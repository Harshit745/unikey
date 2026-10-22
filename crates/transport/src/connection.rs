use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::interval;
use uuid::Uuid;

use device::DeviceIdentity;
use protocol::{Hello, MessageType, Packet, PacketCodec};
use crypto::{IdentityKey, X25519KeyPair, derive_session_keys, derive_session_master, verify_handshake, ReplayGuard, SessionKeys};

use crate::session::{Session, SessionManager};

/// Handshake state machine for secure connection establishment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HandshakeState {
    /// Initial state - waiting to send/receive Hello
    Init,
    /// Sent Hello, waiting for peer's Hello
    SentHello,
    /// Received peer's Hello, need to verify and compute keys
    ReceivedHello,
    /// Handshake complete, encrypted session established
    Established,
    /// Handshake failed
    Failed,
}

/// Context for an ongoing handshake, holding ephemeral keys and peer info.
struct HandshakeContext {
    local_identity: DeviceIdentity,
    local_identity_key: IdentityKey,
    ephemeral_keypair: X25519KeyPair,
    remote_hello: Option<Hello>,
    state: HandshakeState,
}

impl HandshakeContext {
    fn new(local_identity: DeviceIdentity, local_identity_key: IdentityKey) -> Self {
        Self {
            local_identity,
            local_identity_key,
            ephemeral_keypair: X25519KeyPair::generate(),
            remote_hello: None,
            state: HandshakeState::Init,
        }
    }

    /// Current handshake state (for diagnostics).
    fn state(&self) -> HandshakeState {
        self.state
    }

    /// Step 1 — send our signed Hello to the peer. `Init → SentHello`.
    async fn send_hello(
        &mut self,
        stream: &mut TcpStream,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let ephemeral_pub = self.ephemeral_keypair.public_bytes();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let signature = crypto::sign_handshake(
            &self.local_identity_key,
            &self.local_identity.id,
            &ephemeral_pub,
            timestamp,
        );

        let hello = Hello {
            protocol_version: self.local_identity.protocol_version,
            device_id: self.local_identity.id,
            device_name: self.local_identity.name.clone(),
            platform: "Desktop".to_string(),
            ed25519_public_key: self.local_identity_key.public_key(),
            x25519_public_key: ephemeral_pub,
            timestamp,
            signature,
        };

        let payload = serde_json::to_vec(&hello)?;
        let packet = Packet {
            version: 1,
            message_type: MessageType::Hello,
            payload,
        };
        let bytes = PacketCodec::encode(&packet);
        stream.write_all(&bytes).await?;
        self.state = HandshakeState::SentHello;
        Ok(())
    }

    /// Step 2 — read and verify the peer's Hello. `SentHello → ReceivedHello`
    /// then `ReceivedHello → Established` (or `Failed` on signature mismatch).
    async fn receive_and_verify_hello(
        &mut self,
        stream: &mut TcpStream,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let remote_hello = Self::read_hello(stream).await?;
        self.state = HandshakeState::ReceivedHello;

        if !remote_hello.is_secure() {
            self.state = HandshakeState::Failed;
            return Err("Handshake is missing required identity or key-exchange material".into());
        }

        let verified = verify_handshake(
            &remote_hello.ed25519_public_key,
            &remote_hello.device_id,
            &remote_hello.x25519_public_key,
            remote_hello.timestamp,
            &remote_hello.signature,
        );
        if !verified {
            self.state = HandshakeState::Failed;
            return Err("Handshake signature verification failed".into());
        }

        self.remote_hello = Some(remote_hello);
        self.state = HandshakeState::Established;
        Ok(())
    }

    /// Read a raw Hello packet off the wire.
    async fn read_hello(
        stream: &mut TcpStream,
    ) -> Result<Hello, Box<dyn std::error::Error + Send + Sync>> {
        let mut header = [0u8; 6];
        stream.read_exact(&mut header).await?;

        let payload_len_bytes: [u8; 4] = header[2..6].try_into()?;
        let payload_len = u32::from_be_bytes(payload_len_bytes) as usize;

        if payload_len > 1024 * 1024 {
            return Err("Payload exceeds maximum allowed size".into());
        }

        let mut payload = vec![0u8; payload_len];
        stream.read_exact(&mut payload).await?;

        let hello: Hello = serde_json::from_slice(&payload)?;
        Ok(hello)
    }

    /// After `Established`: derive the per-connection session keys.
    fn derive_session_keys(&self) -> Result<SessionKeys, Box<dyn std::error::Error + Send + Sync>> {
        let remote_hello = self
            .remote_hello
            .as_ref()
            .ok_or("Handshake not complete — no remote Hello")?;

        let shared_secret = self
            .ephemeral_keypair
            .shared_secret(&remote_hello.x25519_public_key);
        let master = derive_session_master(
            &shared_secret,
            &self.local_identity.id,
            &remote_hello.device_id,
        );
        Ok(derive_session_keys(
            &master,
            &self.local_identity.id,
            &remote_hello.device_id,
        ))
    }
}

/// Manages TCP connections and performs the initial authentication handshake.
#[derive(Clone)]
pub struct ConnectionManager {
    local_identity: DeviceIdentity,
    local_identity_key: IdentityKey,
    session_manager: SessionManager,
    /// Channel for routing decoded inbound packets to the service layer.
    /// Each message is `(peer_id, decoded_packet)`.
    inbound_tx: Option<mpsc::Sender<(uuid::Uuid, Packet)>>,
}

impl ConnectionManager {
    /// Creates a new ConnectionManager linked to the provided SessionManager.
    ///
    /// `inbound_tx` — optional channel that receives every decoded packet from
    /// all connected peers. The receiver (typically the daemon's service router)
    /// dispatches packets to the appropriate service by `MessageType`.
    pub fn new(
        local_identity: DeviceIdentity,
        local_identity_key: IdentityKey,
        session_manager: SessionManager,
        inbound_tx: Option<mpsc::Sender<(uuid::Uuid, Packet)>>,
    ) -> Self {
        Self {
            local_identity,
            local_identity_key,
            session_manager,
            inbound_tx,
        }
    }

    /// Initiates an outbound TCP connection and handshake to a discovered peer.
    pub async fn connect_to_peer(
        &self,
        addr: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut stream = TcpStream::connect(addr).await?;

        let (session_keys, remote_hello) = self.perform_handshake(&mut stream).await?;

        self.establish_secure_session(stream, remote_hello, session_keys).await;
        Ok(())
    }

    /// Handles an inbound TCP connection from a peer and validates the handshake.
    pub async fn handle_incoming(
        &self,
        mut stream: TcpStream,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (session_keys, remote_hello) = self.perform_handshake(&mut stream).await?;

        self.establish_secure_session(stream, remote_hello, session_keys).await;
        Ok(())
    }

    /// Perform the handshake and return session keys if successful.
    ///
    /// Drives [`HandshakeContext`] through its state machine:
    /// `Init → SentHello → Established`, failing the connection if the peer's
    /// signature does not verify.
    async fn perform_handshake(
        &self,
        stream: &mut TcpStream,
    ) -> Result<(SessionKeys, Hello), Box<dyn std::error::Error + Send + Sync>> {
        let mut ctx = HandshakeContext::new(self.local_identity.clone(), self.local_identity_key.clone());

        // Init → SentHello
        ctx.send_hello(stream).await?;

        // SentHello → Established (or Failed)
        ctx.receive_and_verify_hello(stream).await?;

        let session_keys = ctx.derive_session_keys()?;
        let remote_hello = ctx
            .remote_hello
            .take()
            .ok_or("Handshake completed without a remote Hello")?;

        tracing::debug!(
            "[CONN] Handshake with {} reached {:?}",
            remote_hello.device_name,
            ctx.state()
        );

        Ok((session_keys, remote_hello))
    }

    /// Establish an encrypted session after successful handshake.
    ///
    /// Returns the `JoinHandle` of the connection task, which exits when the
    /// connection drops or the peer disconnects. Awaiting it is how
    /// [`Self::connect_with_reconnect`] detects that a reconnect is needed.
    async fn establish_secure_session(
        &self,
        stream: TcpStream,
        remote_hello: Hello,
        session_keys: SessionKeys,
    ) -> tokio::task::JoinHandle<()> {
        // Outbound channel: service-originated packets consumed by the write task.
        let (tx, mut rx) = mpsc::channel::<Packet>(256);

        // Clone the sender so the read task can send transport-level replies (Pong).
        let pong_tx = tx.clone();

        let peer_id = remote_hello.device_id;
        let connection_id = Uuid::new_v4();

        let session = Session {
            connection_id,
            peer_id,
            peer_name: remote_hello.device_name,
            platform: remote_hello.platform,
            peer_public_key: remote_hello.ed25519_public_key,
            tx,
            session_keys,
            inbound_replay: ReplayGuard::new(),
            outbound_counter: 0,
            last_pong_received: std::time::Instant::now(),
            last_ping_sent: None,
            rtt_ms: None,
            last_activity: std::time::Instant::now(),
        };

        self.session_manager.add_session(session).await;

        let sm = self.session_manager.clone();
        let session_manager_for_cleanup = self.session_manager.clone();
        let inbound_tx = self.inbound_tx.clone();

        tokio::spawn(async move {
            let (mut read_half, mut write_half) = stream.into_split();

            // Shared handle to the session. Every task locks it briefly and
            // releases — no task holds the lock across a blocking I/O.
            let session_arc = match sm.get_session(&peer_id).await {
                Some(s) => s,
                None => {
                    tracing::debug!("[CONN] Session not found for {peer_id}");
                    return;
                }
            };
            let session_arc_for_write = session_arc.clone();

            // ── Read loop ─────────────────────────────────────────────────
            let read_task = async move {
                let mut buf = Vec::with_capacity(4096);

                loop {
                    // Read frame length prefix (4 bytes BE) — no lock held.
                    let mut len_bytes = [0u8; 4];
                    if read_half.read_exact(&mut len_bytes).await.is_err() {
                        tracing::debug!("[CONN] Read frame length failed for {peer_id}");
                        break;
                    }
                    let frame_len = u32::from_be_bytes(len_bytes) as usize;

                    if frame_len > 4 * 1024 * 1024 {
                        tracing::warn!("[CONN] Oversized frame ({} bytes) from {peer_id}", frame_len);
                        break;
                    }

                    // Reuse or resize the read buffer
                    if buf.len() < frame_len {
                        buf.resize(frame_len, 0u8);
                    }
                    let frame_slice = &mut buf[..frame_len];
                    if read_half.read_exact(frame_slice).await.is_err() {
                        tracing::debug!("[CONN] Read frame failed for {peer_id}");
                        break;
                    }

                    // Decrypt and process frame — lock only during processing.
                    {
                        let mut session = session_arc.lock().await;
                        if let Err(e) = process_inbound_frame(
                            frame_slice,
                            &mut session,
                            &inbound_tx,
                            &pong_tx,
                        ).await {
                            tracing::debug!("[CONN] Frame processing error for {peer_id}: {e}");
                            break;
                        }
                    }
                }
            };

            // ── Combined Write + Heartbeat loop ─────────────────────────────
            let write_task = async move {
                let mut ping_interval = interval(Duration::from_secs(30));
                let mut consecutive_missed_pongs = 0u32;
                const MAX_MISSED_PONGS: u32 = 3;

                loop {
                    tokio::select! {
                        // Handle outgoing packets from services
                        packet = rx.recv() => {
                            let Some(packet) = packet else {
                                // All senders dropped → intentional disconnect or
                                // shutdown. Best-effort graceful Disconnect.
                                if let Ok(mut session) = session_arc_for_write.try_lock() {
                                    send_packet(&mut write_half, &mut session, Packet {
                                        version: 1,
                                        message_type: MessageType::Disconnect,
                                        payload: Vec::new(),
                                    }).await;
                                }
                                tracing::debug!("[CONN] Closing write loop for {peer_id}");
                                break;
                            };

                            // Build the encrypted frame under the lock, then
                            // release it before the (possibly slow) socket write.
                            let frame = {
                                let mut session = session_arc_for_write.lock().await;
                                build_outbound_frame(&mut session, packet)
                            };
                            let frame_len = frame.len() as u32;
                            let mut out_buf = Vec::with_capacity(4 + frame.len());
                            out_buf.extend_from_slice(&frame_len.to_be_bytes());
                            out_buf.extend_from_slice(&frame);
                            if write_half.write_all(&out_buf).await.is_err() {
                                tracing::debug!("[CONN] Write failed for {peer_id}");
                                break;
                            }
                        }

                        // Heartbeat: send ping and check for timeout
                        _ = ping_interval.tick() => {
                            let frame = {
                                let mut session = session_arc_for_write.lock().await;
                                let elapsed = session.last_pong_received.elapsed();
                                if elapsed > Duration::from_secs(90) {
                                    consecutive_missed_pongs += 1;
                                    tracing::warn!("[CONN] Missed pong from {peer_id} ({}/{})",
                                        consecutive_missed_pongs, MAX_MISSED_PONGS);
                                    if consecutive_missed_pongs >= MAX_MISSED_PONGS {
                                        tracing::info!("[CONN] Closing connection to {peer_id} — heartbeat timeout");
                                        break;
                                    }
                                } else {
                                    consecutive_missed_pongs = 0;
                                }

                                // Stamp ping send-time for RTT measurement.
                                session.last_ping_sent = Some(std::time::Instant::now());

                                let ping = Packet {
                                    version: 1,
                                    message_type: MessageType::Ping,
                                    payload: Vec::new(),
                                };
                                build_outbound_frame(&mut session, ping)
                            };

                            let frame_len = frame.len() as u32;
                            let mut out_buf = Vec::with_capacity(4 + frame.len());
                            out_buf.extend_from_slice(&frame_len.to_be_bytes());
                            out_buf.extend_from_slice(&frame);
                            if write_half.write_all(&out_buf).await.is_err() {
                                tracing::debug!("[CONN] Failed to send ping to {peer_id}");
                                break;
                            }
                        }
                    }
                }
            };

            // Run both tasks concurrently. The session ends when either side fails.
            tokio::select! {
                _ = read_task => {},
                _ = write_task => {},
            }

            session_manager_for_cleanup
                .remove_session_if_current(&peer_id, connection_id)
                .await;
        })
    }

    /// Connect to a peer and keep the connection alive across transient network
    /// failures using exponential backoff.
    ///
    /// Every successful handshake spawns a session task; when that task exits
    /// (peer went away, heartbeat timeout, or network blip) and reconnection
    /// has not been cancelled, this retries. The loop stops when `cancel`
    /// fires (daemon shutdown or intentional disconnect).
    ///
    /// # Note
    /// If the peer's address changes (new DHCP lease, machine sleep/wake), the
    /// caller should re-enter with the fresh address — UDP discovery keeps
    /// `PeerManager` up to date.
    pub async fn connect_with_reconnect(
        &self,
        addr: SocketAddr,
        peer_id: Uuid,
        cancel: tokio_util::sync::CancellationToken,
    ) {
        let mut attempt: u32 = 0;
        loop {
            if cancel.is_cancelled() {
                tracing::info!("[CONN] Reconnect cancelled for {peer_id}");
                return;
            }

            // If we already have a live session, nothing to do.
            if self.session_manager.has_session(&peer_id).await {
                // Wait until that session ends.
                loop {
                    tokio::select! {
                        _ = cancel.cancelled() => return,
                        _ = tokio::time::sleep(Duration::from_secs(1)) => {
                            if !self.session_manager.has_session(&peer_id).await {
                                break;
                            }
                        }
                    }
                }
            }

            // Exponential backoff: 1s, 2s, 4s, … capped at 30s.
            let backoff = Duration::from_secs((1u32 << attempt.min(5)).min(30) as u64);
            attempt += 1;

            tokio::select! {
                _ = cancel.cancelled() => return,
                _ = tokio::time::sleep(backoff) => {}
            }

            tracing::info!(
                "[CONN] Reconnecting to {peer_id} ({addr}) — attempt {attempt}"
            );
            let result = self.connect_to_peer(addr).await;
            match result {
                Ok(()) => {
                    tracing::info!("[CONN] Reconnected to {peer_id}");
                    attempt = 0;
                }
                Err(e) => {
                    tracing::warn!(
                        "[CONN] Reconnect to {peer_id} failed: {e} — retrying in {backoff:?}"
                    );
                }
            }
        }
    }

    /// Gracefully disconnect from a peer: requests the session's write loop to
    /// emit a `Disconnect` packet and then tears the session down.
    pub async fn disconnect(&self, peer_id: &Uuid) {
        if let Some(tx) = self.session_manager.get_session_sender(peer_id).await {
            // Dropping the sender closes the write loop, which sends Disconnect.
            drop(tx);
        }
        self.session_manager.remove_session(peer_id).await;
        tracing::info!("[CONN] Disconnected from {peer_id}");
    }
}

/// Write a single encrypted packet frame to the wire (best-effort).
async fn send_packet(
    write_half: &mut tokio::net::tcp::OwnedWriteHalf,
    session: &mut Session,
    packet: Packet,
) {
    let frame = build_outbound_frame(session, packet);
    let frame_len = frame.len() as u32;
    let mut out_buf = Vec::with_capacity(4 + frame.len());
    out_buf.extend_from_slice(&frame_len.to_be_bytes());
    out_buf.extend_from_slice(&frame);
    if write_half.write_all(&out_buf).await.is_err() {
        tracing::debug!("[CONN] send_packet failed for {}", session.peer_id);
    }
}

/// Process an inbound encrypted frame: decrypt, verify replay, decode packet.
async fn process_inbound_frame(
    frame_bytes: &[u8],
    session: &mut Session,
    inbound_tx: &Option<mpsc::Sender<(uuid::Uuid, Packet)>>,
    pong_tx: &mpsc::Sender<Packet>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use protocol::{EncryptedFrame, FRAME_HEADER_LEN, FRAME_VERSION, FLAG_ENCRYPTED, MessageType, Packet, PacketCodec};
    use crypto::{open, Nonce};

    if frame_bytes.len() < FRAME_HEADER_LEN {
        return Err("Frame too short".into());
    }
    if frame_bytes[0] != FRAME_VERSION {
        return Err(format!("Unsupported frame version {}", frame_bytes[0]).into());
    }
    if frame_bytes[1] != FLAG_ENCRYPTED {
        return Err("Frame not encrypted".into());
    }

    let frame = EncryptedFrame::decode(frame_bytes)?;

    // Any valid frame is activity: refreshes the idle-timeout window.
    session.last_activity = std::time::Instant::now();

    // Replay check
    if !session.inbound_replay.check_and_commit(frame.counter) {
        return Err(format!("Replay detected: counter {}", frame.counter).into());
    }

    // Decrypt
    let aad = frame.aad();
    let plaintext = open(
        &session.session_keys.recv_key,
        &Nonce(frame.nonce),
        &aad,
        &frame.ciphertext,
    )?;

    // Decode inner packet
    let packet = PacketCodec::decode(&plaintext)?;

    // Handle transport-level messages
    match packet.message_type {
        MessageType::Ping => {
            let pong = Packet {
                version: 1,
                message_type: MessageType::Pong,
                payload: Vec::new(),
            };
            // try_send: we hold the session lock here and must never block on a
            // full channel (the write loop would wait on the same lock).
            if pong_tx.try_send(pong).is_err() {
                tracing::trace!("[CONN] Dropped Pong reply — outbound channel full");
            }
        }
        MessageType::Pong => {
            // Transport-level keep-alive ack - update last pong received time
            // and measure round-trip time.
            session.last_pong_received = std::time::Instant::now();
            if let Some(sent) = session.last_ping_sent.take() {
                session.rtt_ms = Some(sent.elapsed().as_millis() as u64);
            }
        }
        MessageType::Disconnect => {
            tracing::info!("[CONN] Peer {} sent Disconnect", session.peer_id);
            return Err("Peer disconnected".into());
        }
        MessageType::Hello => {
            tracing::warn!("[CONN] Unexpected Hello from {}", session.peer_id);
        }
        _ => {
            // Route all other packets to the service layer
            if let Some(tx) = inbound_tx
                && tx.try_send((session.peer_id, packet)).is_err()
            {
                tracing::warn!("[CONN] Inbound channel full — dropping packet for {}", session.peer_id);
            }
        }
    }

    Ok(())
}

/// Build an outbound encrypted frame from a plaintext packet.
fn build_outbound_frame(session: &mut Session, packet: Packet) -> Vec<u8> {
    use protocol::{EncryptedFrame, FLAG_ENCRYPTED, PacketCodec, FRAME_VERSION};
    use crypto::{seal, Nonce};
    use rand_core::{OsRng, RngCore};

    // Encode the inner packet
    let plaintext = PacketCodec::encode(&packet);

    // Increment counter
    session.outbound_counter += 1;
    let counter = session.outbound_counter;

    // Generate nonce: counter (8 bytes) + 4 random bytes
    let mut random = [0u8; 4];
    OsRng.fill_bytes(&mut random);
    let nonce = Nonce::from_counter(counter, random);

    // Build AAD
    let mut aad = [0u8; 10];
    aad[0] = FRAME_VERSION;
    aad[1] = FLAG_ENCRYPTED;
    aad[2..10].copy_from_slice(&counter.to_be_bytes());

    // Encrypt
    let ciphertext_with_tag = seal(&session.session_keys.send_key, &nonce, &aad, &plaintext);

    // Build frame
    let frame = EncryptedFrame {
        counter,
        nonce: *nonce.as_bytes(),
        ciphertext: ciphertext_with_tag,
    };

    frame.encode()
}
