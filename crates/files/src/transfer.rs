use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Default chunk size for file transfers (64 KiB). Each chunk is a separate
/// `FileChunk` packet, so this bounds per-packet memory and lets large files
/// stream without loading them into RAM.
pub const CHUNK_SIZE: usize = 64 * 1024;

/// Hard cap on a single transferred file (8 GiB). Prevents a malicious or
/// buggy peer from advertising an absurd size that would exhaust disk/RAM.
pub const MAX_FILE_SIZE: u64 = 8 * 1024 * 1024 * 1024;

/// Maximum length of a transferred filename (matches common FS limits).
pub const MAX_FILENAME_LEN: usize = 255;

/// Whether this device is sending or receiving the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferDirection {
    Send,
    Receive,
}

/// Lifecycle state of a transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferStatus {
    /// Registered but no chunks exchanged yet.
    Queued,
    /// At least one chunk transferred.
    InProgress,
    /// All chunks transferred and integrity verified.
    Completed,
    /// Acknowledged by the remote peer as verified.
    Verified,
    /// Failed (I/O, checksum mismatch, peer error).
    Failed,
    /// Explicitly cancelled by either side.
    Cancelled,
}

/// Metadata describing a file to be transferred.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// Unique identifier for this transfer.
    pub transfer_id: Uuid,
    /// Original filename (sanitized before use on the receiving side).
    pub filename: String,
    /// Total file size in bytes.
    pub total_size: u64,
    /// SHA-256 of the complete file, for integrity verification.
    pub checksum: [u8; 32],
    /// Number of chunks (ceil(total_size / CHUNK_SIZE)).
    pub total_chunks: u32,
}

/// A single chunk of file data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChunk {
    pub transfer_id: Uuid,
    /// Zero-based index of this chunk.
    pub chunk_index: u32,
    /// Raw bytes for this chunk (≤ CHUNK_SIZE).
    pub data: Vec<u8>,
}

/// Public snapshot of a transfer for the API / GUI.
#[derive(Debug, Clone, Serialize)]
pub struct TransferInfo {
    pub transfer_id: Uuid,
    pub peer_id: Uuid,
    pub filename: String,
    pub total_size: u64,
    pub transferred: u64,
    pub direction: TransferDirection,
    pub status: TransferStatus,
    pub progress_percent: f32,
    pub error: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Internal per-transfer bookkeeping.
#[derive(Debug)]
struct TransferState {
    peer_id: Uuid,
    metadata: FileMetadata,
    direction: TransferDirection,
    status: TransferStatus,
    /// Bytes received so far (receiver) or bytes read off disk (sender).
    transferred: u64,
    error: Option<String>,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    /// Receiver: which chunk indices have arrived (dedup / ordering).
    received_chunks: Vec<bool>,
    /// Receiver: reassembled bytes, removed when the transfer completes.
    buffer: Vec<u8>,
}

/// Registry of all in-flight and finished transfers, shared across tasks.
#[derive(Debug, Clone)]
pub struct TransferManager {
    inner: Arc<Mutex<HashMap<Uuid, TransferState>>>,
}

impl TransferManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a new outgoing transfer (this device sends the file).
    pub async fn register_outgoing(
        &self,
        peer_id: Uuid,
        metadata: FileMetadata,
    ) -> Result<Uuid, anyhow::Error> {
        let mut map = self.inner.lock().await;
        map.insert(
            metadata.transfer_id,
            TransferState {
                peer_id,
                metadata: metadata.clone(),
                direction: TransferDirection::Send,
                status: TransferStatus::Queued,
                transferred: 0,
                error: None,
                started_at: Utc::now(),
                completed_at: None,
                received_chunks: Vec::new(),
                buffer: Vec::new(),
            },
        );
        Ok(metadata.transfer_id)
    }

    /// Register a new incoming transfer. Returns an error if the metadata is
    /// invalid (oversized file, bad chunk count) so the receiver can reject
    /// the transfer before buffering anything.
    pub async fn register_incoming(
        &self,
        peer_id: Uuid,
        metadata: &FileMetadata,
    ) -> Result<(), anyhow::Error> {
        if metadata.total_size > MAX_FILE_SIZE {
            anyhow::bail!(
                "file too large: {} bytes (max {MAX_FILE_SIZE})",
                metadata.total_size
            );
        }
        // The chunk count must match the declared size.
        let expected_chunks = metadata.total_size.div_ceil(CHUNK_SIZE as u64);
        if expected_chunks != metadata.total_chunks as u64 {
            anyhow::bail!(
                "chunk count mismatch: declared {} for {} bytes, expected {expected_chunks}",
                metadata.total_chunks,
                metadata.total_size
            );
        }
        let mut map = self.inner.lock().await;
        if map.contains_key(&metadata.transfer_id) {
            anyhow::bail!("duplicate transfer {}", metadata.transfer_id);
        }
        map.insert(
            metadata.transfer_id,
            TransferState {
                peer_id,
                metadata: metadata.clone(),
                direction: TransferDirection::Receive,
                status: TransferStatus::Queued,
                transferred: 0,
                error: None,
                started_at: Utc::now(),
                completed_at: None,
                received_chunks: vec![false; metadata.total_chunks as usize],
                buffer: Vec::with_capacity(metadata.total_size as usize),
            },
        );
        Ok(())
    }

    /// Update the transferred-byte counter (sender side, after each read).
    pub async fn record_sent(&self, transfer_id: Uuid, total_sent: u64) {
        let mut map = self.inner.lock().await;
        if let Some(state) = map.get_mut(&transfer_id) {
            state.transferred = total_sent;
            if state.status == TransferStatus::Queued {
                state.status = TransferStatus::InProgress;
            }
        }
    }

    /// Record receipt of a chunk. Returns `Ok(Some((metadata, bytes)))` when
    /// the transfer is complete and the reassembled file is ready (the entry
    /// is removed from the registry).
    ///
    /// Validation happens here: the chunk index must be in range, the chunk
    /// must not exceed CHUNK_SIZE, and each index may only arrive once.
    pub async fn receive_chunk(
        &self,
        peer_id: &Uuid,
        chunk: &FileChunk,
    ) -> anyhow::Result<Option<(FileMetadata, Vec<u8>)>> {
        let mut map = self.inner.lock().await;
        let Some(state) = map.get_mut(&chunk.transfer_id) else {
            anyhow::bail!("unknown transfer {}", chunk.transfer_id);
        };
        if state.peer_id != *peer_id {
            anyhow::bail!(
                "transfer {} was opened by {}, not {peer_id}",
                chunk.transfer_id,
                state.peer_id
            );
        }
        if !matches!(
            state.status,
            TransferStatus::Queued | TransferStatus::InProgress
        ) {
            anyhow::bail!(
                "transfer {} is {:?} — no further chunks accepted",
                chunk.transfer_id,
                state.status
            );
        }
        let idx = chunk.chunk_index as usize;
        if idx >= state.received_chunks.len() {
            anyhow::bail!(
                "chunk index {idx} out of range ({} chunks)",
                state.received_chunks.len()
            );
        }
        if chunk.data.len() > CHUNK_SIZE {
            anyhow::bail!(
                "chunk {} too large: {} bytes (max {CHUNK_SIZE})",
                chunk.chunk_index,
                chunk.data.len()
            );
        }
        // Bounds check: this chunk must fit within the declared file size.
        let chunk_offset = idx as u64 * CHUNK_SIZE as u64;
        if chunk_offset + chunk.data.len() as u64 > state.metadata.total_size {
            anyhow::bail!(
                "chunk {} overruns declared size {}",
                chunk.chunk_index,
                state.metadata.total_size
            );
        }
        if state.received_chunks[idx] {
            // Duplicate chunk — ignore (idempotent delivery), not an error.
            return Ok(None);
        }
        state.received_chunks[idx] = true;
        state.buffer.extend_from_slice(&chunk.data);
        state.transferred += chunk.data.len() as u64;
        state.status = TransferStatus::InProgress;

        let done = state.received_chunks.iter().all(|&r| r);
        if done {
            // Take the reassembled bytes out of the entry (freeing the memory)
            // but KEEP the entry so the service can record a terminal status.
            // The service verifies the checksum and calls mark_completed /
            // mark_failed afterwards.
            let bytes = std::mem::take(&mut state.buffer);
            state.transferred = state.metadata.total_size;
            return Ok(Some((state.metadata.clone(), bytes)));
        }
        Ok(None)
    }

    /// Mark a transfer as completed (all chunks sent, or integrity verified).
    pub async fn mark_completed(&self, transfer_id: Uuid, verified: bool) {
        let mut map = self.inner.lock().await;
        if let Some(state) = map.get_mut(&transfer_id) {
            state.status = if verified {
                TransferStatus::Verified
            } else {
                TransferStatus::Completed
            };
            state.completed_at = Some(Utc::now());
            state.transferred = state.metadata.total_size;
        }
    }

    /// Mark a transfer as failed, recording the reason.
    pub async fn mark_failed(&self, transfer_id: Uuid, error: String) {
        let mut map = self.inner.lock().await;
        if let Some(state) = map.get_mut(&transfer_id) {
            state.status = TransferStatus::Failed;
            state.error = Some(error);
            state.completed_at = Some(Utc::now());
        }
    }

    /// Cancel a transfer (either side). Returns false if it was already gone.
    pub async fn cancel(&self, transfer_id: Uuid) -> bool {
        let mut map = self.inner.lock().await;
        if let Some(state) = map.get_mut(&transfer_id) {
            if matches!(
                state.status,
                TransferStatus::Completed | TransferStatus::Verified | TransferStatus::Cancelled
            ) {
                return false;
            }
            state.status = TransferStatus::Cancelled;
            state.completed_at = Some(Utc::now());
            true
        } else {
            false
        }
    }

    /// Fetch a single transfer snapshot.
    pub async fn get(&self, transfer_id: Uuid) -> Option<TransferInfo> {
        let map = self.inner.lock().await;
        map.get(&transfer_id).map(to_info)
    }

    /// Snapshot of every transfer, most recently started first.
    pub async fn list(&self) -> Vec<TransferInfo> {
        let map = self.inner.lock().await;
        let mut all: Vec<TransferInfo> = map.values().map(to_info).collect();
        all.sort_by_key(|t| std::cmp::Reverse(t.started_at));
        all
    }

    /// True if the transfer was cancelled (so the sender can stop early).
    pub async fn is_cancelled(&self, transfer_id: Uuid) -> bool {
        let map = self.inner.lock().await;
        matches!(
            map.get(&transfer_id).map(|s| s.status),
            Some(TransferStatus::Cancelled)
        )
    }
}

impl Default for TransferManager {
    fn default() -> Self {
        Self::new()
    }
}

fn to_info(state: &TransferState) -> TransferInfo {
    let total = state.metadata.total_size.max(1);
    let progress = (state.transferred as f32 / total as f32).min(1.0);
    TransferInfo {
        transfer_id: state.metadata.transfer_id,
        peer_id: state.peer_id,
        filename: state.metadata.filename.clone(),
        total_size: state.metadata.total_size,
        transferred: state.transferred,
        direction: state.direction,
        status: state.status,
        progress_percent: progress * 100.0,
        error: state.error.clone(),
        started_at: Some(state.started_at),
        completed_at: state.completed_at,
    }
}

/// Sanitize a peer-supplied filename before it touches the filesystem.
///
/// Returns `None` when the name cannot be used safely. Guards against
/// directory traversal (`..`, absolute paths, separators) and control chars.
pub fn sanitize_filename(name: &str) -> Option<String> {
    let name = name.trim();
    if name.is_empty() || name.len() > MAX_FILENAME_LEN {
        return None;
    }
    if name == "." || name == ".." || name.starts_with('.') {
        return None;
    }
    // Neutralize any path separator so the name can never escape its folder.
    let sanitized = name.replace(['/', '\\'], "_");
    if sanitized == "." || sanitized == ".." {
        return None;
    }
    if sanitized.chars().any(|c| c.is_control()) {
        return None;
    }
    Some(sanitized)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(transfer_id: Uuid, size: u64) -> FileMetadata {
        FileMetadata {
            transfer_id,
            filename: "hello.txt".into(),
            total_size: size,
            checksum: [0u8; 32],
            total_chunks: size.div_ceil(CHUNK_SIZE as u64) as u32,
        }
    }

    #[tokio::test]
    async fn reassembles_a_file_across_chunks() {
        let mgr = TransferManager::new();
        let id = Uuid::new_v4();
        let data: Vec<u8> = (0..150_000u32).map(|i| (i % 251) as u8).collect();

        let peer = Uuid::new_v4();
        mgr.register_incoming(peer, &metadata(id, data.len() as u64))
            .await
            .unwrap();

        let mut result = None;
        for (idx, chunk_data) in data.chunks(CHUNK_SIZE).enumerate() {
            let chunk = FileChunk {
                transfer_id: id,
                chunk_index: idx as u32,
                data: chunk_data.to_vec(),
            };
            if let Some((_meta, buf)) = mgr.receive_chunk(&peer, &chunk).await.unwrap() {
                result = Some(buf);
            }
        }
        assert_eq!(result, Some(data));
    }

    #[tokio::test]
    async fn rejects_out_of_range_chunks() {
        let mgr = TransferManager::new();
        let id = Uuid::new_v4();
        mgr.register_incoming(Uuid::new_v4(), &metadata(id, 100))
            .await
            .unwrap();

        let peer = Uuid::new_v4();
        let bad = FileChunk {
            transfer_id: id,
            chunk_index: 5, // only 1 chunk exists
            data: vec![0u8; 10],
        };
        assert!(mgr.receive_chunk(&peer, &bad).await.is_err());
    }

    #[tokio::test]
    async fn rejects_chunks_from_wrong_peer() {
        let mgr = TransferManager::new();
        let id = Uuid::new_v4();
        mgr.register_incoming(Uuid::new_v4(), &metadata(id, 100))
            .await
            .unwrap();

        let wrong_peer = Uuid::new_v4();
        let chunk = FileChunk {
            transfer_id: id,
            chunk_index: 0,
            data: vec![0u8; 10],
        };
        assert!(mgr.receive_chunk(&wrong_peer, &chunk).await.is_err());
    }

    #[tokio::test]
    async fn rejects_oversized_files() {
        let mgr = TransferManager::new();
        let id = Uuid::new_v4();
        let mut meta = metadata(id, MAX_FILE_SIZE + 1);
        meta.total_chunks = (MAX_FILE_SIZE + 1).div_ceil(CHUNK_SIZE as u64) as u32;
        assert!(mgr.register_incoming(Uuid::new_v4(), &meta).await.is_err());
    }

    #[tokio::test]
    async fn rejects_chunk_count_mismatch() {
        let mgr = TransferManager::new();
        let id = Uuid::new_v4();
        let mut meta = metadata(id, 1000);
        meta.total_chunks = 99; // wrong
        assert!(mgr.register_incoming(Uuid::new_v4(), &meta).await.is_err());
    }

    #[tokio::test]
    async fn cancel_flips_status_and_blocks_completion() {
        let mgr = TransferManager::new();
        let id = Uuid::new_v4();
        let peer = Uuid::new_v4();
        mgr.register_incoming(peer, &metadata(id, 100)).await.unwrap();
        assert!(mgr.cancel(id).await);
        // After cancel, a late chunk is treated as an unknown transfer.
        let chunk = FileChunk {
            transfer_id: id,
            chunk_index: 0,
            data: vec![0u8; 10],
        };
        assert!(mgr.receive_chunk(&peer, &chunk).await.is_err());
    }

    #[test]
    fn sanitizes_hostile_filenames() {
        assert_eq!(sanitize_filename("report.pdf"), Some("report.pdf".into()));
        assert_eq!(sanitize_filename("  spaced.txt "), Some("spaced.txt".into()));
        // Traversal attempts — leading-dot names (incl. "..") are rejected.
        assert_eq!(sanitize_filename("../etc/passwd"), None);
        assert_eq!(sanitize_filename(".."), None);
        assert_eq!(sanitize_filename("/absolute/path"), Some("_absolute_path".into()));
        // A name that merely *contains* a separator is neutralized, not rejected.
        assert_eq!(sanitize_filename("a/../b.txt"), Some("a_.._b.txt".into()));
        // Hidden files and controls.
        assert_eq!(sanitize_filename(".bashrc"), None);
        assert_eq!(sanitize_filename("bad\u{0000}name"), None);
        assert_eq!(sanitize_filename(""), None);
        let long: String = "a".repeat(MAX_FILENAME_LEN + 1);
        assert_eq!(sanitize_filename(&long), None);
    }
}
