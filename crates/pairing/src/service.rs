use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{Mutex, oneshot};
use uuid::Uuid;

use anyhow::{bail, Result};
use device::DeviceIdentity;
use protocol::{MessageType, Packet};
use services::{Phase, Service, ServiceContext};
use transport::session::SessionManager;

use crate::session::{PairingSession, PairingSessionManager, PairingState};
use crate::trust_store::{TrustStore, TrustedDevice};

/// Default TTL for pairing sessions in seconds.
const PAIRING_TTL_SECS: u64 = 60;

/// Result of a pairing initiation handshake received via PairResponse.
#[derive(Debug, Clone)]
pub struct PairingHandshakeResult {
    pub accepted: bool,
    pub peer_id: Option<Uuid>,
    pub peer_name: Option<String>,
    pub peer_platform: Option<String>,
    pub initiation_session_id: Uuid,
}

/// The Pairing Service manages device trust relationships.
#[derive(Debug)]
pub struct PairingService {
    trust_store: Arc<Mutex<TrustStore>>,
    session_manager: PairingSessionManager,
    /// Oneshot channels awaiting PairResponse for a given initiation session.
    pending_responses: Arc<Mutex<HashMap<Uuid, oneshot::Sender<PairingHandshakeResult>>>>,
    /// Runtime collaborators supplied when the service starts. Keeping these
    /// here lets an inbound PairRequest send its correlated PairResponse over
    /// the already-established encrypted transport session.
    runtime: Arc<Mutex<Option<PairingRuntime>>>,
}

#[derive(Debug, Clone)]
struct PairingRuntime {
    local_device: DeviceIdentity,
    session_manager: SessionManager,
}

#[derive(Debug, serde::Deserialize)]
struct PairRequestPayload {
    pair_code: String,
    initiator_id: Uuid,
    initiator_name: String,
    initiator_platform: String,
    initiation_session_id: Uuid,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PairResponsePayload {
    accepted: bool,
    responder_id: Uuid,
    responder_name: String,
    responder_platform: String,
    initiation_session_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl PairingService {
    /// Build a service backed by the OS config directory (`trusted.json`).
    pub fn new() -> Result<Self> {
        Self::with_store_root(None)
    }

    /// Build a service whose trust store lives under an explicit data root
    /// (portable release folder, tests). `None` uses the OS config directory.
    pub fn with_store_root(root: Option<&std::path::Path>) -> Result<Self> {
        let store = match root {
            Some(root) => TrustStore::load_at(root)?,
            None => TrustStore::load()?,
        };
        Ok(Self {
            trust_store: Arc::new(Mutex::new(store)),
            session_manager: PairingSessionManager::new(),
            pending_responses: Arc::new(Mutex::new(HashMap::new())),
            runtime: Arc::new(Mutex::new(None)),
        })
    }

    // ── Session generation ────────────────────────────────────────────

    /// Generate a new pairing session and return its data for the Flutter UI.
    pub async fn generate_session(&self, device_id: Uuid) -> PairingSession {
        self.session_manager
            .create_session(device_id, PAIRING_TTL_SECS)
            .await
    }

    /// Get the current state of a pairing session (for status polling).
    pub async fn pairing_status(&self, session_id: &Uuid) -> Option<PairingSession> {
        self.session_manager.get_session(session_id).await
    }

    // ── Code validation (incoming PairRequest handling) ────────────────

    /// Validate an incoming pair code and return the matching session, if any.
    pub async fn validate_code(&self, code: &str) -> Option<PairingSession> {
        let session = self.session_manager.find_by_code(code).await?;
        if session.is_expired() {
            self.session_manager
                .update_state(&session.session_id, PairingState::Expired)
                .await;
            return None;
        }
        Some(session)
    }

    /// Mark a session as accepted by the remote peer.
    pub async fn accept_session(&self, session_id: &Uuid, remote_id: Uuid) {
        self.session_manager
            .update_state(session_id, PairingState::Accepted(remote_id))
            .await;
    }

    /// Complete the pairing: add the remote device to the trust store.
    pub async fn complete_pairing(
        &self,
        session_id: &Uuid,
        remote_id: Uuid,
        remote_name: &str,
        remote_platform: &str,
        remote_identity_key: &[u8; 32],
    ) -> Result<()> {
        let mut store = self.trust_store.lock().await;
        store.add_trusted(
            remote_id,
            remote_name,
            remote_platform,
            remote_identity_key,
        );
        store.record_event(
            crate::trust_store::PairingEventKind::Paired,
            remote_id,
            remote_name,
        );
        store.save()?;
        tracing::info!("[PAIRING] Paired with device: {remote_name} ({remote_id})");
        self.session_manager
            .update_state(session_id, PairingState::Completed)
            .await;
        Ok(())
    }

    // ── Response routing (outbound PairRequest → inbound PairResponse) ─

    /// Register a oneshot channel that will receive the PairResponse result
    /// for an outbound pairing initiation. The caller supplies the exact ID
    /// placed on the wire, preventing a response-correlation mismatch.
    pub async fn register_pending_response(
        &self,
        initiation_session_id: Uuid,
        tx: oneshot::Sender<PairingHandshakeResult>,
    ) {
        self.pending_responses
            .lock()
            .await
            .insert(initiation_session_id, tx);
    }

    /// Remove an abandoned waiter after connection/send failure or timeout.
    pub async fn cancel_pending_response(&self, initiation_session_id: &Uuid) {
        self.pending_responses
            .lock()
            .await
            .remove(initiation_session_id);
    }

    /// Resolve a pending pairing response (called when PairResponse arrives).
    pub async fn resolve_pending_response(
        &self,
        initiation_session_id: &Uuid,
        result: PairingHandshakeResult,
    ) {
        let tx = self
            .pending_responses
            .lock()
            .await
            .remove(initiation_session_id);
        if let Some(tx) = tx {
            let _ = tx.send(result);
        }
    }

    // ── Device management ─────────────────────────────────────────────

    /// Remove a trusted device from the store.
    pub async fn remove_trusted_device(&self, id: &Uuid) -> Result<()> {
        let mut store = self.trust_store.lock().await;
        if let Some(device) = store.trusted.iter().find(|d| &d.id == id).cloned() {
            store.record_event(
                crate::trust_store::PairingEventKind::Removed,
                device.id,
                &device.name,
            );
        }
        store.remove_trusted(id);
        store.save()?;
        tracing::info!("[PAIRING] Removed trusted device: {id}");
        Ok(())
    }

    /// Return the pairing history (most recent first).
    pub async fn pairing_history(&self) -> Vec<crate::trust_store::PairingEvent> {
        let store = self.trust_store.lock().await;
        store.history()
    }

    /// Rename (alias) a trusted device.
    pub async fn rename_trusted_device(&self, id: &Uuid, new_name: &str) -> Result<()> {
        let mut store = self.trust_store.lock().await;
        if !store.rename_device(id, new_name) {
            bail!("Device not found in trust store");
        }
        store.save()?;
        tracing::info!("[PAIRING] Renamed device {id} to {new_name}");
        Ok(())
    }

    /// Return all currently trusted devices.
    pub async fn trusted_devices(&self) -> Vec<TrustedDevice> {
        let store = self.trust_store.lock().await;
        store.list().to_vec()
    }

    /// Check if a device is trusted.
    pub async fn is_trusted(&self, peer_id: &Uuid) -> bool {
        let store = self.trust_store.lock().await;
        store.is_trusted(peer_id)
    }

    /// Authorize protected service traffic only when the live transport
    /// session presents the same Ed25519 identity key pinned during pairing.
    pub async fn is_current_session_trusted(&self, peer_id: &Uuid) -> bool {
        let runtime = match self.runtime.lock().await.clone() {
            Some(runtime) => runtime,
            None => return false,
        };
        let peer_key = match runtime.session_manager.get_peer_public_key(peer_id).await {
            Some(key) => key,
            None => return false,
        };
        self.trust_store
            .lock()
            .await
            .is_trusted_identity(peer_id, &peer_key)
    }

    async fn current_peer_identity_key(&self, peer_id: &Uuid) -> Result<[u8; 32]> {
        let runtime = self
            .runtime
            .lock()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("pairing service is not started"))?;
        runtime
            .session_manager
            .get_peer_public_key(peer_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("no authenticated session with peer {peer_id}"))
    }

    /// Clean up expired pairing sessions.
    pub async fn cleanup_expired_sessions(&self) -> usize {
        self.session_manager.cleanup_expired().await
    }

    async fn send_pair_response(
        &self,
        peer_id: &Uuid,
        initiation_session_id: Uuid,
        accepted: bool,
        error: Option<String>,
    ) -> Result<()> {
        let runtime = self
            .runtime
            .lock()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("pairing service is not started"))?;
        let sender = runtime
            .session_manager
            .get_session_sender(peer_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("no active session with peer {peer_id}"))?;
        let response = PairResponsePayload {
            accepted,
            responder_id: runtime.local_device.id,
            responder_name: runtime.local_device.name,
            responder_platform: runtime.local_device.platform.to_string(),
            initiation_session_id,
            error,
        };
        sender
            .send(Packet {
                version: common::constants::PROTOCOL_VERSION as u8,
                message_type: MessageType::PairResponse,
                payload: serde_json::to_vec(&response)?,
            })
            .await
            .map_err(|e| anyhow::anyhow!("failed to queue PairResponse: {e}"))
    }
}

#[async_trait]
impl Service for PairingService {
    fn name(&self) -> &'static str {
        "pairing"
    }

    fn phase(&self) -> Phase {
        Phase::Phase1
    }

    async fn start(&self, ctx: ServiceContext) -> Result<()> {
        *self.runtime.lock().await = Some(PairingRuntime {
            local_device: ctx.local_device.clone(),
            session_manager: ctx.session_manager.clone(),
        });
        let sm = self.session_manager.clone();
        let shutdown = ctx.shutdown.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let removed = sm.cleanup_expired().await;
                        if removed > 0 {
                            tracing::trace!("[PAIRING] Cleaned up {removed} expired session(s)");
                        }
                    }
                    _ = shutdown.cancelled() => break,
                }
            }
        });
        tracing::info!(
            "[PAIRING] Service started — {} trusted device(s)",
            self.trusted_devices().await.len()
        );
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    async fn handle_packet(
        &self,
        peer_id: &Uuid,
        msg_type: MessageType,
        payload: &[u8],
    ) -> Result<()> {
        match msg_type {
            MessageType::PairRequest => {
                // Remote device is trying to pair with us using a code.
                let req: PairRequestPayload = serde_json::from_slice(payload)?;

                if req.initiator_id != *peer_id {
                    tracing::warn!(
                        "[PAIRING] Rejecting identity mismatch: transport peer {peer_id}, payload {}",
                        req.initiator_id
                    );
                    self.send_pair_response(
                        peer_id,
                        req.initiation_session_id,
                        false,
                        Some("initiator identity does not match transport session".into()),
                    )
                    .await?;
                    return Ok(());
                }

                let session = match self.validate_code(&req.pair_code).await {
                    Some(s) => s,
                    None => {
                        tracing::warn!("[PAIRING] Invalid/expired pair code from {peer_id}");
                        self.send_pair_response(
                            peer_id,
                            req.initiation_session_id,
                            false,
                            Some("invalid or expired pairing code".into()),
                        )
                        .await?;
                        return Ok(());
                    }
                };

                tracing::info!(
                    "[PAIRING] PairRequest from {peer_id} ({}) — code valid",
                    req.initiator_name
                );

                self.accept_session(&session.session_id, *peer_id).await;

                // Add remote to trust store
                let peer_identity_key = self.current_peer_identity_key(peer_id).await?;
                self.complete_pairing(
                    &session.session_id,
                    *peer_id,
                    &req.initiator_name,
                    &req.initiator_platform,
                    &peer_identity_key,
                )
                .await?;

                self.send_pair_response(
                    peer_id,
                    req.initiation_session_id,
                    true,
                    None,
                )
                .await?;
            }
            MessageType::PairResponse => {
                // Our outgoing PairRequest was answered.
                let resp: PairResponsePayload = serde_json::from_slice(payload)?;

                if resp.responder_id != *peer_id {
                    bail!(
                        "PairResponse identity mismatch: transport peer {peer_id}, payload {}",
                        resp.responder_id
                    );
                }

                if resp.accepted {
                    let peer_identity_key = self.current_peer_identity_key(peer_id).await?;
                    self.complete_pairing(
                        &Uuid::nil(), // outbound pairing has no local code session
                        resp.responder_id,
                        &resp.responder_name,
                        &resp.responder_platform,
                        &peer_identity_key,
                    )
                    .await?;

                    tracing::info!(
                        "[PAIRING] PairResponse — accepted by {} ({})",
                        resp.responder_name,
                        resp.responder_id
                    );
                } else {
                    tracing::warn!(
                        "[PAIRING] PairResponse — rejected by {}: {}",
                        resp.responder_name,
                        resp.error.as_deref().unwrap_or("no reason supplied")
                    );
                }

                self.resolve_pending_response(
                    &resp.initiation_session_id,
                    PairingHandshakeResult {
                        accepted: resp.accepted,
                        peer_id: Some(resp.responder_id),
                        peer_name: Some(resp.responder_name),
                        peer_platform: Some(resp.responder_platform),
                        initiation_session_id: resp.initiation_session_id,
                    },
                )
                .await;
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A fresh per-test trust-store root, cleaned up on drop.
    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "unikey-pairing-{tag}-{}",
            Uuid::new_v4()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[tokio::test]
    async fn generated_code_validates_and_expires() {
        let root = temp_root("code");
        let svc = PairingService::with_store_root(Some(&root)).unwrap();
        let device = Uuid::new_v4();

        let session = svc.generate_session(device).await;
        assert_eq!(session.pair_code.len(), 6);
        assert_eq!(session.state, PairingState::Pending);
        assert_eq!(session.local_device_id, device);

        // The code resolves back to the same session.
        let found = svc.validate_code(&session.pair_code).await.expect("valid code");
        assert_eq!(found.session_id, session.session_id);

        // A wrong code never matches.
        assert!(svc.validate_code("000000").await.is_none());
        assert!(svc.validate_code("").await.is_none());
    }

    #[tokio::test]
    async fn expired_code_is_rejected_and_marked_expired() {
        let root = temp_root("expiry");
        let svc = PairingService::with_store_root(Some(&root)).unwrap();

        // A session with a 1-second TTL.
        let session = svc.session_manager.create_session(Uuid::new_v4(), 1).await;
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        assert!(svc.validate_code(&session.pair_code).await.is_none());
        let s = svc.pairing_status(&session.session_id).await.unwrap();
        assert_eq!(s.state, PairingState::Expired);
    }

    #[tokio::test]
    async fn completing_pairing_trusts_and_persists_the_peer() {
        let root = temp_root("complete");
        let svc = PairingService::with_store_root(Some(&root)).unwrap();
        let device = Uuid::new_v4();
        let peer = Uuid::new_v4();

        let session = svc.generate_session(device).await;
        svc.complete_pairing(
            &session.session_id,
            peer,
            "peer-laptop",
            "Windows",
            &[7; 32],
        )
            .await
            .unwrap();

        // Trust is live in memory…
        assert!(svc.is_trusted(&peer).await);
        assert_eq!(svc.trusted_devices().await.len(), 1);
        assert_eq!(svc.pairing_status(&session.session_id).await.unwrap().state, PairingState::Completed);

        // …and survives a reload from the same root.
        let reloaded = PairingService::with_store_root(Some(&root)).unwrap();
        assert!(reloaded.is_trusted(&peer).await);
    }

    #[tokio::test]
    async fn history_records_removal() {
        let root = temp_root("history");
        let svc = PairingService::with_store_root(Some(&root)).unwrap();
        let device = Uuid::new_v4();
        let peer = Uuid::new_v4();

        let session = svc.generate_session(device).await;
        svc.complete_pairing(
            &session.session_id,
            peer,
            "laptop",
            "macOS",
            &[7; 32],
        )
        .await
        .unwrap();
        svc.remove_trusted_device(&peer).await.unwrap();

        assert!(!svc.is_trusted(&peer).await);
        let events = svc.pairing_history().await;
        // Most recent first: Removed, then Paired.
        assert_eq!(events[0].kind, crate::trust_store::PairingEventKind::Removed);
        assert_eq!(events[1].kind, crate::trust_store::PairingEventKind::Paired);
    }

    #[tokio::test]
    async fn renaming_unknown_device_is_an_error() {
        let root = temp_root("rename");
        let svc = PairingService::with_store_root(Some(&root)).unwrap();
        assert!(svc.rename_trusted_device(&Uuid::new_v4(), "x").await.is_err());

        let peer = Uuid::new_v4();
        svc.complete_pairing(
            &Uuid::new_v4(),
            peer,
            "laptop",
            "Linux",
            &[7; 32],
        )
        .await
        .unwrap();
        svc.rename_trusted_device(&peer, "home-rig").await.unwrap();
        let d = svc.trusted_devices().await.into_iter().find(|d| d.id == peer).unwrap();
        assert_eq!(d.alias.as_deref(), Some("home-rig"));
    }

    #[tokio::test]
    async fn pending_responses_route_the_result() {
        let root = temp_root("pending");
        let svc = PairingService::with_store_root(Some(&root)).unwrap();
        let (tx, rx) = oneshot::channel();
        let sid = Uuid::new_v4();
        svc.register_pending_response(sid, tx).await;

        let result = PairingHandshakeResult {
            accepted: true,
            peer_id: Some(Uuid::new_v4()),
            peer_name: Some("peer".into()),
            peer_platform: Some("Windows".into()),
            initiation_session_id: sid,
        };
        svc.resolve_pending_response(&sid, result.clone()).await;
        assert_eq!(rx.await.unwrap().peer_name, result.peer_name);

        // Resolving an unknown id is a no-op (no panic).
        svc.resolve_pending_response(&Uuid::new_v4(), result).await;
    }

    fn identity(id: Uuid, name: &str, platform: device::Platform) -> DeviceIdentity {
        DeviceIdentity {
            id,
            name: name.into(),
            platform,
            device_type: device::DeviceType::Desktop,
            created_at: chrono::Utc::now(),
            protocol_version: common::constants::PROTOCOL_VERSION,
            public_key_path: String::new(),
            private_key_path: String::new(),
        }
    }

    async fn add_test_session(
        manager: &SessionManager,
        peer: &DeviceIdentity,
    ) -> tokio::sync::mpsc::Receiver<Packet> {
        let (tx, rx) = tokio::sync::mpsc::channel(8);
        manager
            .add_session(transport::session::Session {
                connection_id: Uuid::new_v4(),
                peer_id: peer.id,
                peer_name: peer.name.clone(),
                platform: peer.platform.to_string(),
                peer_public_key: [9; 32],
                tx,
                session_keys: crypto::SessionKeys {
                    send_key: [1; 32],
                    recv_key: [2; 32],
                },
                inbound_replay: crypto::ReplayGuard::new(),
                outbound_counter: 0,
                last_pong_received: std::time::Instant::now(),
                last_ping_sent: None,
                rtt_ms: None,
                last_activity: std::time::Instant::now(),
            })
            .await;
        rx
    }

    #[tokio::test]
    async fn pair_request_response_completes_trust_on_both_sides() {
        let alice = identity(Uuid::new_v4(), "alice-mac", device::Platform::MacOS);
        let bob = identity(Uuid::new_v4(), "bob-linux", device::Platform::Linux);
        let alice_root = temp_root("roundtrip-alice");
        let bob_root = temp_root("roundtrip-bob");
        let alice_service = PairingService::with_store_root(Some(&alice_root)).unwrap();
        let bob_service = PairingService::with_store_root(Some(&bob_root)).unwrap();
        let alice_sessions = SessionManager::new();
        let bob_sessions = SessionManager::new();
        let _alice_outbound = add_test_session(&alice_sessions, &bob).await;
        let mut bob_outbound = add_test_session(&bob_sessions, &alice).await;

        alice_service
            .start(ServiceContext::new(
                alice.clone(),
                alice_sessions.clone(),
                tokio_util::sync::CancellationToken::new(),
            ))
            .await
            .unwrap();
        bob_service
            .start(ServiceContext::new(
                bob.clone(),
                bob_sessions,
                tokio_util::sync::CancellationToken::new(),
            ))
            .await
            .unwrap();

        let bob_pairing = bob_service.generate_session(bob.id).await;
        let initiation_session_id = Uuid::new_v4();
        let (result_tx, result_rx) = oneshot::channel();
        alice_service
            .register_pending_response(initiation_session_id, result_tx)
            .await;

        let request = serde_json::json!({
            "pair_code": bob_pairing.pair_code,
            "initiator_id": alice.id,
            "initiator_name": alice.name,
            "initiator_platform": alice.platform.to_string(),
            "initiation_session_id": initiation_session_id,
        });
        bob_service
            .handle_packet(
                &alice.id,
                MessageType::PairRequest,
                &serde_json::to_vec(&request).unwrap(),
            )
            .await
            .unwrap();

        let response = bob_outbound.recv().await.expect("PairResponse queued");
        assert_eq!(response.message_type, MessageType::PairResponse);
        alice_service
            .handle_packet(&bob.id, response.message_type, &response.payload)
            .await
            .unwrap();

        let result = result_rx.await.expect("initiator receives response");
        assert!(result.accepted);
        assert_eq!(result.peer_id, Some(bob.id));
        assert!(alice_service.is_trusted(&bob.id).await);
        assert!(bob_service.is_trusted(&alice.id).await);
        assert!(alice_service.is_current_session_trusted(&bob.id).await);
        assert!(bob_service.is_current_session_trusted(&alice.id).await);

        // Reusing a trusted UUID with another identity key must not authorize
        // protected service traffic.
        alice_sessions
            .get_session(&bob.id)
            .await
            .unwrap()
            .lock()
            .await
            .peer_public_key = [10; 32];
        assert!(!alice_service.is_current_session_trusted(&bob.id).await);
    }
}
