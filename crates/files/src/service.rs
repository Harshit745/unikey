use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;
use tokio::sync::RwLock;
use uuid::Uuid;

use anyhow::{Context, Result};
use events::{AppEvent, EventBus};
use protocol::{MessageType, Packet};
use services::{Phase, Service, ServiceContext};
use transport::session::SessionManager;

use crate::transfer::{
    sanitize_filename, FileChunk, FileMetadata, TransferInfo, TransferManager, CHUNK_SIZE,
    MAX_FILE_SIZE,
};

/// Default folder for received files when the daemon does not override it.
fn default_download_dir() -> PathBuf {
    dirs::download_dir().unwrap_or_else(std::env::temp_dir)
}

/// The File Transfer Service enables peer-to-peer file sharing between
/// connected and trusted devices.
///
/// Flow (sender): the API calls [`send_file`](Self::send_file) → the service
/// computes a SHA-256 of the file, broadcasts `FileMetadata`, then streams
/// `FileChunk` packets of 64 KiB. Flow (receiver): `handle_packet` validates
/// every chunk, reassembles, verifies the SHA-256, writes the file atomically
/// into the download directory and sends back `FileAck`. Either side can
/// abort with `FileCancel`.
///
/// Security:
/// - Untrusted peers never reach this service (the daemon router gates on the
///   trust store first).
/// - Incoming filenames are sanitized (no traversal, no absolute paths).
/// - File size and per-chunk sizes are validated before buffering.
/// - Chunks are only accepted for a registered transfer from the peer that
///   opened it, in-range, and never duplicated.
/// - The complete file is SHA-256 verified before it is written, and the
///   write is atomic (temp file + rename).
#[derive(Debug)]
pub struct FileTransferService {
    manager: TransferManager,
    download_dir: PathBuf,
    /// Session manager, captured at `start()` so the API can send after the
    /// service is live. Stored as an `Option` in an `RwLock` so the service
    /// can be constructed (and Debug-derived) before startup.
    session_manager: Arc<RwLock<Option<SessionManager>>>,
    /// Live event bus, captured at `start()`.
    events: Arc<RwLock<Option<EventBus>>>,
}

impl FileTransferService {
    pub fn new() -> Self {
        Self::with_download_dir(default_download_dir())
    }

    pub fn with_download_dir(dir: impl Into<PathBuf>) -> Self {
        Self {
            manager: TransferManager::new(),
            download_dir: dir.into(),
            session_manager: Arc::new(RwLock::new(None)),
            events: Arc::new(RwLock::new(None)),
        }
    }

    /// Snapshot of every transfer, newest first (for the API / GUI).
    pub async fn list(&self) -> Vec<TransferInfo> {
        self.manager.list().await
    }

    /// Fetch a single transfer.
    pub async fn get(&self, transfer_id: Uuid) -> Option<TransferInfo> {
        self.manager.get(transfer_id).await
    }

    /// Cancel a transfer and notify the peer, so both sides stop early.
    pub async fn cancel(&self, transfer_id: Uuid) -> Result<()> {
        let info = self
            .manager
            .get(transfer_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("transfer {transfer_id} not found"))?;
        if !self.manager.cancel(transfer_id).await {
            return Ok(()); // already terminal — nothing to do
        }
        if let Some(sm) = self.session_manager().await {
            send_control(&sm, &info.peer_id, MessageType::FileCancel, transfer_id).await;
        }
        if let Some(bus) = self.events.read().await.clone() {
            bus.publish(AppEvent::TransferCancelled { transfer_id });
        }
        tracing::info!("[FILES] Cancelled transfer {transfer_id}");
        Ok(())
    }

    /// Begin sending `path` to `peer_id`. Returns the new transfer id.
    ///
    /// The file is hashed once up front (so `FileMetadata` carries the true
    /// SHA-256), then streamed chunk-by-chunk from a background task so the
    /// API call returns immediately and the GUI can poll progress.
    pub async fn send_file(&self, peer_id: &Uuid, path: &Path) -> Result<Uuid> {
        let sm = self
            .session_manager()
            .await
            .ok_or_else(|| anyhow::anyhow!("file service is not started"))?;
        if !sm.has_session(peer_id).await {
            anyhow::bail!("no active session with peer {peer_id}");
        }

        let meta = tokio::fs::metadata(path)
            .await
            .with_context(|| format!("cannot stat {}", path.display()))?;
        if !meta.is_file() {
            anyhow::bail!("{} is not a regular file", path.display());
        }
        let size = meta.len();
        if size > MAX_FILE_SIZE {
            anyhow::bail!("file too large: {size} bytes (max {MAX_FILE_SIZE})");
        }
        let raw_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unnamed".to_string());
        let filename = sanitize_filename(&raw_name)
            .ok_or_else(|| anyhow::anyhow!("unsafe filename: {raw_name}"))?;

        let checksum = compute_sha256(path).await?;
        let total_chunks = size.div_ceil(CHUNK_SIZE as u64) as u32;
        let transfer_id = Uuid::new_v4();
        let metadata = FileMetadata {
            transfer_id,
            filename,
            total_size: size,
            checksum,
            total_chunks,
        };

        self.manager
            .register_outgoing(*peer_id, metadata.clone())
            .await?;

        let events = self.events.read().await.clone();
        if let Some(bus) = events.as_ref() {
            bus.publish(AppEvent::TransferStarted {
                transfer_id,
                peer_id: *peer_id,
                filename: metadata.filename.clone(),
                total_size: metadata.total_size,
                direction: "send".to_string(),
            });
        }

        let manager = self.manager.clone();
        tokio::spawn(chunk_and_send(
            *peer_id,
            path.to_path_buf(),
            metadata,
            manager,
            sm,
            events,
        ));
        tracing::info!("[FILES] Queued {} ({size} bytes) to {peer_id}", path.display());
        Ok(transfer_id)
    }

    async fn session_manager(&self) -> Option<SessionManager> {
        self.session_manager.read().await.clone()
    }

    /// Atomically write a verified incoming file into the download directory.
    async fn save_incoming(&self, filename: &str, bytes: &[u8]) -> Result<PathBuf> {
        let name = sanitize_filename(filename).context("unsafe incoming filename")?;
        let dir = self.download_dir.clone();
        tokio::fs::create_dir_all(&dir).await?;

        let final_path = dir.join(&name);
        // Defense in depth: even with a sanitized name, make sure the target
        // stays inside the download directory.
        let canonical_dir = tokio::fs::canonicalize(&dir).await?;
        let parent = final_path.parent().context("filename has no parent")?;
        if tokio::fs::canonicalize(parent).await? != canonical_dir {
            anyhow::bail!("incoming file escapes download directory");
        }

        // Write to a temp name and rename, so a crash or kill mid-write never
        // leaves a half-written file under its final name.
        let part_path = dir.join(format!(".{name}.part"));
        tokio::fs::write(&part_path, bytes).await?;
        tokio::fs::rename(&part_path, &final_path).await?;
        Ok(final_path)
    }
}

impl Default for FileTransferService {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute the SHA-256 of a file (used up front for `FileMetadata`).
async fn compute_sha256(path: &Path) -> Result<[u8; 32]> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; CHUNK_SIZE];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().into())
}

/// Stream a file to a peer in fixed-size chunks, hashing as we go so the
/// digest is guaranteed to match the bytes actually sent.
async fn chunk_and_send(
    peer_id: Uuid,
    path: PathBuf,
    metadata: FileMetadata,
    manager: TransferManager,
    sm: SessionManager,
    events: Option<EventBus>,
) {
    let transfer_id = metadata.transfer_id;
    let total = metadata.total_size;

    // Announce the transfer before streaming any chunks so the receiver can
    // register it and reject invalid metadata up front.
    let meta_payload = match serde_json::to_vec(&metadata) {
        Ok(p) => p,
        Err(e) => {
            fail_transfer(&manager, transfer_id, format!("serialize error: {e}"), &events).await;
            return;
        }
    };
    let meta_packet = Packet {
        version: 1,
        message_type: MessageType::FileMetadata,
        payload: meta_payload,
    };
    match sm.get_session_sender(&peer_id).await {
        Some(tx) => {
            if let Err(e) = tx.send(meta_packet).await {
                fail_transfer(&manager, transfer_id, format!("send error: {e}"), &events).await;
                return;
            }
        }
        None => {
            fail_transfer(&manager, transfer_id, "peer session closed before transfer".to_string(), &events).await;
            return;
        }
    }

    let mut file = match tokio::fs::File::open(&path).await {
        Ok(f) => f,
        Err(e) => {
            fail_transfer(&manager, transfer_id, format!("cannot open file: {e}"), &events).await;
            return;
        }
    };

    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut chunk_index = 0u32;
    let mut sent = 0u64;
    // Throttle progress events to ~10 Hz so the GUI isn't flooded on big files.
    let mut last_progress = std::time::Instant::now();

    loop {
        // Honour an early cancel from either side.
        if manager.is_cancelled(transfer_id).await {
            send_control(&sm, &peer_id, MessageType::FileCancel, transfer_id).await;
            if let Some(bus) = &events {
                bus.publish(AppEvent::TransferCancelled { transfer_id });
            }
            return;
        }

        let n = match file.read(&mut buf).await {
            Ok(n) => n,
            Err(e) => {
                fail_transfer(&manager, transfer_id, format!("read error: {e}"), &events).await;
                return;
            }
        };
        if n == 0 {
            break;
        }

        hasher.update(&buf[..n]);
        sent += n as u64;

        let chunk = FileChunk {
            transfer_id,
            chunk_index,
            data: buf[..n].to_vec(),
        };
        let payload = match serde_json::to_vec(&chunk) {
            Ok(p) => p,
            Err(e) => {
                fail_transfer(&manager, transfer_id, format!("serialize error: {e}"), &events).await;
                return;
            }
        };
        let packet = Packet {
            version: 1,
            message_type: MessageType::FileChunk,
            payload,
        };

        match sm.get_session_sender(&peer_id).await {
            Some(tx) => {
                if let Err(e) = tx.send(packet).await {
                    fail_transfer(&manager, transfer_id, format!("send error: {e}"), &events).await;
                    return;
                }
            }
            None => {
                fail_transfer(&manager, transfer_id, "peer session closed during transfer".to_string(), &events).await;
                return;
            }
        }

        manager.record_sent(transfer_id, sent).await;
        if let Some(bus) = &events
            && last_progress.elapsed() >= std::time::Duration::from_millis(100)
        {
            bus.publish(AppEvent::TransferProgress {
                transfer_id,
                transferred: sent,
                total,
            });
            last_progress = std::time::Instant::now();
        }
        chunk_index += 1;
    }

    // The digest of what we actually sent must match what we advertised.
    let digest: [u8; 32] = hasher.finalize().into();
    if digest != metadata.checksum {
        fail_transfer(&manager, transfer_id, "local checksum mismatch during send".to_string(), &events).await;
        return;
    }

    manager.mark_completed(transfer_id, false).await;
    if let Some(bus) = &events {
        bus.publish(AppEvent::TransferCompleted {
            transfer_id,
            filename: metadata.filename.clone(),
            verified: false,
        });
    }
    tracing::info!("[FILES] Sent {} to {peer_id}", metadata.filename);
}

/// Mark a transfer failed and publish the failure event.
async fn fail_transfer(
    manager: &TransferManager,
    transfer_id: Uuid,
    error: String,
    events: &Option<EventBus>,
) {
    manager.mark_failed(transfer_id, error.clone()).await;
    if let Some(bus) = events {
        bus.publish(AppEvent::TransferFailed { transfer_id, error });
    }
}

/// Send a FileAck / FileCancel control packet carrying a transfer id.
async fn send_control(sm: &SessionManager, peer_id: &Uuid, msg_type: MessageType, transfer_id: Uuid) {
    let payload = match serde_json::to_vec(&transfer_id) {
        Ok(p) => p,
        Err(_) => return,
    };
    let packet = Packet {
        version: 1,
        message_type: msg_type,
        payload,
    };
    if let Some(tx) = sm.get_session_sender(peer_id).await {
        let _ = tx.send(packet).await;
    }
}

#[async_trait]
impl Service for FileTransferService {
    fn name(&self) -> &'static str {
        "files"
    }

    fn phase(&self) -> Phase {
        Phase::Phase1
    }

    async fn start(&self, ctx: ServiceContext) -> Result<()> {
        *self.session_manager.write().await = Some(ctx.session_manager.clone());
        *self.events.write().await = Some(ctx.events.clone());
        tokio::fs::create_dir_all(&self.download_dir).await.with_context(|| {
            format!(
                "unable to create download directory {}",
                self.download_dir.display()
            )
        })?;
        tracing::info!(
            "[FILES] Service started — receiving into {}",
            self.download_dir.display()
        );
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        tracing::info!("[FILES] Service stopped");
        Ok(())
    }

    async fn handle_packet(
        &self,
        peer_id: &Uuid,
        msg_type: MessageType,
        payload: &[u8],
    ) -> Result<()> {
        let sm = match self.session_manager().await {
            Some(sm) => sm,
            None => return Ok(()), // not started yet — nothing we can reply with
        };

        match msg_type {
            MessageType::FileMetadata => {
                let meta: FileMetadata = serde_json::from_slice(payload)?;
                if sanitize_filename(&meta.filename).is_none() {
                    tracing::warn!("[FILES] Rejected unsafe filename from {peer_id}");
                    send_control(&sm, peer_id, MessageType::FileCancel, meta.transfer_id).await;
                    return Ok(());
                }
                if let Err(e) = self.manager.register_incoming(*peer_id, &meta).await {
                    tracing::warn!("[FILES] Rejected transfer {} from {peer_id}: {e}", meta.transfer_id);
                    send_control(&sm, peer_id, MessageType::FileCancel, meta.transfer_id).await;
                    return Ok(());
                }
                tracing::info!(
                    "[FILES] Receiving {} ({} bytes) from {peer_id}",
                    meta.filename,
                    meta.total_size
                );
            }

            MessageType::FileChunk => {
                let chunk: FileChunk = serde_json::from_slice(payload)?;
                match self.manager.receive_chunk(peer_id, &chunk).await {
                    Ok(Some((meta, bytes))) => {
                        // Complete transfer — verify integrity, then persist.
                        let digest: [u8; 32] = Sha256::digest(&bytes).into();
                        if digest != meta.checksum {
                            tracing::warn!(
                                "[FILES] Checksum mismatch for {} from {peer_id}",
                                meta.filename
                            );
                            self.manager
                                .mark_failed(
                                    meta.transfer_id,
                                    "checksum mismatch (data corrupted in transit)".to_string(),
                                )
                                .await;
                            if let Some(bus) = self.events.read().await.clone() {
                                bus.publish(AppEvent::TransferFailed {
                                    transfer_id: meta.transfer_id,
                                    error: "checksum mismatch (data corrupted in transit)"
                                        .to_string(),
                                });
                            }
                            send_control(
                                &sm,
                                peer_id,
                                MessageType::FileCancel,
                                meta.transfer_id,
                            )
                            .await;
                            return Ok(());
                        }
                        match self.save_incoming(&meta.filename, &bytes).await {
                            Ok(saved) => {
                                self.manager
                                    .mark_completed(meta.transfer_id, true)
                                    .await;
                                send_control(&sm, peer_id, MessageType::FileAck, meta.transfer_id)
                                    .await;
                                if let Some(bus) = self.events.read().await.clone() {
                                    bus.publish(AppEvent::TransferCompleted {
                                        transfer_id: meta.transfer_id,
                                        filename: meta.filename.clone(),
                                        verified: true,
                                    });
                                }
                                tracing::info!(
                                    "[FILES] Received {} from {peer_id} → {}",
                                    meta.filename,
                                    saved.display()
                                );
                            }
                            Err(e) => {
                                self.manager
                                    .mark_failed(meta.transfer_id, e.to_string())
                                    .await;
                                if let Some(bus) = self.events.read().await.clone() {
                                    bus.publish(AppEvent::TransferFailed {
                                        transfer_id: meta.transfer_id,
                                        error: e.to_string(),
                                    });
                                }
                                send_control(
                                    &sm,
                                    peer_id,
                                    MessageType::FileCancel,
                                    meta.transfer_id,
                                )
                                .await;
                            }
                        }
                    }
                    Ok(None) => {} // still receiving
                    Err(e) => {
                        tracing::warn!("[FILES] Bad chunk from {peer_id}: {e}");
                        send_control(&sm, peer_id, MessageType::FileCancel, chunk.transfer_id)
                            .await;
                    }
                }
            }

            MessageType::FileAck => {
                if let Ok(transfer_id) = serde_json::from_slice::<Uuid>(payload) {
                    self.manager.mark_completed(transfer_id, true).await;
                    tracing::info!("[FILES] Peer {peer_id} verified transfer {transfer_id}");
                }
            }

            MessageType::FileCancel => {
                if let Ok(transfer_id) = serde_json::from_slice::<Uuid>(payload) {
                    if self.manager.cancel(transfer_id).await
                        && let Some(bus) = self.events.read().await.clone()
                    {
                        bus.publish(AppEvent::TransferCancelled { transfer_id });
                    }
                    tracing::info!("[FILES] Peer {peer_id} cancelled transfer {transfer_id}");
                }
            }

            _ => {
                tracing::debug!("[FILES] Unhandled packet type from {peer_id}: {msg_type:?}");
            }
        }
        Ok(())
    }
}
