use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use uuid::Uuid;

use protocol::Packet;
use crypto::{SessionKeys, ReplayGuard};
use events::{AppEvent, EventBus};

/// Represents an active, authenticated session with a remote device.
///
/// Sessions are stored in the [`SessionManager`] behind `Arc<Mutex<..>>` so the
/// transport's read loop, write loop and heartbeat task can all touch the same
/// session without racing on check-out/check-in.
#[derive(Debug)]
pub struct Session {
    /// Unique token for this concrete TCP connection. It prevents an older
    /// connection task from removing a newer replacement session for the same
    /// peer when the old socket finally closes.
    pub connection_id: Uuid,
    pub peer_id: Uuid,
    pub peer_name: String,
    pub platform: String,
    /// Long-term Ed25519 identity key authenticated by the transport Hello.
    /// The pairing trust store pins this key to `peer_id`.
    pub peer_public_key: [u8; 32],
    /// Wire optimization advertised by the peer, not a trust decision.
    pub file_chunk_binary: bool,
    /// Channel used by feature modules (Clipboard, etc.) to send outbound packets.
    pub tx: mpsc::Sender<Packet>,
    /// Session keys for encrypted communication
    pub session_keys: SessionKeys,
    /// Inbound replay guard
    pub inbound_replay: ReplayGuard,
    /// Outbound frame counter
    pub outbound_counter: u64,
    /// Last time we received a Pong from the peer (for heartbeat timeout).
    pub last_pong_received: std::time::Instant,
    /// When the most recent Ping was sent (to measure round-trip time).
    pub last_ping_sent: Option<std::time::Instant>,
    /// Measured round-trip time in ms (None until first Pong).
    pub rtt_ms: Option<u64>,
    /// Last time any activity occurred on this session (for idle timeout).
    pub last_activity: std::time::Instant,
}

/// Thread-safe manager for all active device sessions on the network.
#[derive(Debug, Clone, Default)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<Uuid, Arc<Mutex<Session>>>>>,
    /// Optional live event bus. When set (by the daemon), session
    /// connect/disconnect events are published so the GUI dashboard can react
    /// in real time without polling.
    events: Arc<RwLock<Option<EventBus>>>,
}

impl SessionManager {
    /// Creates a new, empty Session Manager.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            events: Arc::new(RwLock::new(None)),
        }
    }

    /// Attach the shared event bus. Sessions created before this call produce
    /// no events; the daemon calls this once at startup.
    pub async fn set_event_bus(&self, bus: EventBus) {
        *self.events.write().await = Some(bus);
    }

    /// Registers a new authenticated session and makes it available for feature routing.
    pub async fn add_session(&self, session: Session) {
        let peer_id = session.peer_id;
        let peer_name = session.peer_name.clone();
        let mut lock = self.sessions.write().await;
        tracing::info!("[SESSION] Connected to: {peer_name} ({peer_id})");
        lock.insert(peer_id, Arc::new(Mutex::new(session)));
        drop(lock);

        if let Some(bus) = self.events.read().await.clone() {
            bus.publish(AppEvent::SessionConnected { peer_id, peer_name });
        }
    }

    /// Removes a session when the connection drops or the device goes offline.
    pub async fn remove_session(&self, peer_id: &Uuid) {
        let mut lock = self.sessions.write().await;
        if let Some(session) = lock.remove(peer_id) {
            let guard = session.lock().await;
            let peer_name = guard.peer_name.clone();
            tracing::info!(
                "[SESSION] Disconnected from: {} ({})",
                guard.peer_name,
                peer_id
            );
            drop(guard);
            drop(lock);

            if let Some(bus) = self.events.read().await.clone() {
                bus.publish(AppEvent::SessionDisconnected {
                    peer_id: *peer_id,
                    peer_name,
                });
            }
        }
    }

    /// Remove a session only if it still belongs to the specified connection.
    /// Returns `true` when a session was removed.
    pub async fn remove_session_if_current(
        &self,
        peer_id: &Uuid,
        connection_id: Uuid,
    ) -> bool {
        let mut lock = self.sessions.write().await;
        let peer_name = match lock.get(peer_id) {
            Some(session) => {
                let session = session.lock().await;
                if session.connection_id != connection_id {
                    return false;
                }
                session.peer_name.clone()
            }
            None => return false,
        };
        lock.remove(peer_id);
        drop(lock);
        if let Some(bus) = self.events.read().await.clone() {
            bus.publish(AppEvent::SessionDisconnected {
                peer_id: *peer_id,
                peer_name,
            });
        }
        true
    }

    /// Checks if a session with the given peer UUID is already active.
    pub async fn has_session(&self, peer_id: &Uuid) -> bool {
        let lock = self.sessions.read().await;
        lock.contains_key(peer_id)
    }

    /// Retrieves the transmission channel for a specific peer to send outbound packets.
    pub async fn get_session_sender(&self, peer_id: &Uuid) -> Option<mpsc::Sender<Packet>> {
        let lock = self.sessions.read().await;
        let session = lock.get(peer_id)?;
        Some(session.lock().await.tx.clone())
    }

    /// Keep encoding and sender bound to the same concrete session.
    pub async fn file_transfer_sender(&self, peer_id: &Uuid) -> Option<(mpsc::Sender<Packet>, bool)> {
        let session = self.get_session(peer_id).await?;
        let session = session.lock().await;
        Some((session.tx.clone(), session.file_chunk_binary))
    }

    /// Return the authenticated long-term identity key for an active peer.
    pub async fn get_peer_public_key(&self, peer_id: &Uuid) -> Option<[u8; 32]> {
        let lock = self.sessions.read().await;
        let session = lock.get(peer_id)?;
        Some(session.lock().await.peer_public_key)
    }

    /// Get a shared handle to a session. The returned `Arc<Mutex<Session>>`
    /// stays valid for the session's lifetime; callers lock it briefly, do
    /// their work, and release. Returns `None` if the session is gone.
    pub async fn get_session(&self, peer_id: &Uuid) -> Option<Arc<Mutex<Session>>> {
        let lock = self.sessions.read().await;
        lock.get(peer_id).cloned()
    }

    /// Broadcast a packet to every active session.
    /// Saturation-safe: if a peer's channel is full the packet is dropped for that peer
    /// rather than blocking the broadcaster.
    pub async fn broadcast_packet(&self, packet: Packet) {
        let lock = self.sessions.read().await;
        for session in lock.values() {
            let guard = session.lock().await;
            if guard.tx.try_send(packet.clone()).is_err() {
                tracing::trace!(
                    "[SESSION] Dropped packet for {} — channel full",
                    guard.peer_id
                );
            }
        }
    }

    /// Returns the number of currently active sessions.
    pub async fn session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Returns the peer IDs of all currently active sessions.
    pub async fn peer_ids(&self) -> Vec<Uuid> {
        self.sessions.read().await.keys().copied().collect()
    }

    /// Returns a snapshot of every active session's public metadata.
    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let lock = self.sessions.read().await;
        let mut out = Vec::with_capacity(lock.len());
        for session in lock.values() {
            let guard = session.lock().await;
            out.push(SessionInfo {
                peer_id: guard.peer_id,
                peer_name: guard.peer_name.clone(),
                platform: guard.platform.clone(),
                idle_for_ms: guard.last_activity.elapsed().as_millis() as u64,
                rtt_ms: guard.rtt_ms,
            });
        }
        out
    }

    /// Remove sessions whose last activity is older than `idle_timeout`.
    ///
    /// Returns the set of peer IDs that were dropped so callers can log them.
    /// The transport's own read loop usually tears a connection down first; this
    /// is a backstop for sessions orphaned by a crashed task.
    pub async fn cleanup_idle_sessions(
        &self,
        idle_timeout: std::time::Duration,
    ) -> Vec<Uuid> {
        let now = std::time::Instant::now();

        // Gather stale candidates without blocking on any busy session.
        let stale: Vec<Uuid> = {
            let lock = self.sessions.read().await;
            let mut out = Vec::new();
            for (peer_id, session) in lock.iter() {
                if let Ok(g) = session.try_lock()
                    && now.duration_since(g.last_activity) > idle_timeout
                {
                    out.push(*peer_id);
                }
                // A session held by a task mid-frame is not reaped.
            }
            out
        };

        // Remove the stale ones (allowing an await since the write lock is held
        // briefly and the per-session locks are try_locked).
        let mut expired = Vec::new();
        if !stale.is_empty() {
            let mut lock = self.sessions.write().await;
            for peer_id in stale {
                if let Some(session) = lock.remove(&peer_id) {
                    if let Ok(g) = session.try_lock() {
                        tracing::info!(
                            "[SESSION] Idle timeout — dropping session {peer_id} ({})",
                            g.peer_name
                        );
                    }
                    expired.push(peer_id);
                }
            }
        }
        expired
    }

    /// Spawn a background task that periodically sweeps idle sessions.
    ///
    /// `interval` controls how often the sweep runs (e.g. 60s), and
    /// `idle_timeout` is the inactivity threshold after which a session is
    /// dropped. The task runs until the runtime shuts down.
    pub fn spawn_idle_sweeper(
        self,
        interval: std::time::Duration,
        idle_timeout: std::time::Duration,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                let dropped = self.cleanup_idle_sessions(idle_timeout).await;
                if !dropped.is_empty() {
                    tracing::warn!(
                        "[SESSION] Idle sweeper removed {} stale session(s): {:?}",
                        dropped.len(),
                        dropped
                    );
                }
            }
        })
    }
}

/// A public, serialisable snapshot of an active session.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionInfo {
    pub peer_id: Uuid,
    pub peer_name: String,
    pub platform: String,
    pub idle_for_ms: u64,
    pub rtt_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal, fully-formed session for lifecycle tests.
    fn dummy_session(peer_id: Uuid, peer_name: &str) -> Session {
        let (tx, _rx) = mpsc::channel(16);
        Session {
            connection_id: Uuid::new_v4(),
            peer_id,
            peer_name: peer_name.to_string(),
            platform: "test".to_string(),
            peer_public_key: [7; 32],
            file_chunk_binary: false,
            tx,
            session_keys: SessionKeys {
                send_key: [0u8; 32],
                recv_key: [0u8; 32],
            },
            inbound_replay: ReplayGuard::new(),
            outbound_counter: 0,
            last_pong_received: std::time::Instant::now(),
            last_ping_sent: None,
            rtt_ms: None,
            last_activity: std::time::Instant::now(),
        }
    }

    #[tokio::test]
    async fn publishes_connect_and_disconnect_events() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let sm = SessionManager::new();
        sm.set_event_bus(bus).await;

        let peer_id = Uuid::new_v4();
        sm.add_session(dummy_session(peer_id, "bob")).await;

        let env = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("connect event within timeout")
            .expect("channel open");
        match env.event {
            AppEvent::SessionConnected { peer_id: got, peer_name } => {
                assert_eq!(got, peer_id);
                assert_eq!(peer_name, "bob");
            }
            other => panic!("unexpected event {other:?}"),
        }

        sm.remove_session(&peer_id).await;
        let env = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("disconnect event within timeout")
            .expect("channel open");
        match env.event {
            AppEvent::SessionDisconnected { peer_id: got, .. } => assert_eq!(got, peer_id),
            other => panic!("unexpected event {other:?}"),
        }
    }

    #[tokio::test]
    async fn stale_connection_cannot_remove_newer_replacement() {
        let manager = SessionManager::new();
        let peer = Uuid::new_v4();
        let old = dummy_session(peer, "old");
        let old_connection = old.connection_id;
        manager.add_session(old).await;

        let replacement = dummy_session(peer, "replacement");
        let replacement_connection = replacement.connection_id;
        manager.add_session(replacement).await;

        assert!(
            !manager
                .remove_session_if_current(&peer, old_connection)
                .await,
            "stale cleanup must not remove the replacement"
        );
        assert!(manager.has_session(&peer).await);
        assert!(
            manager
                .remove_session_if_current(&peer, replacement_connection)
                .await
        );
        assert!(!manager.has_session(&peer).await);
    }

    #[tokio::test]
    async fn lifecycle_works_without_an_event_bus() {
        let sm = SessionManager::new();
        let peer_id = Uuid::new_v4();
        sm.add_session(dummy_session(peer_id, "bob")).await;
        assert!(sm.has_session(&peer_id).await);
        assert_eq!(sm.session_count().await, 1);
        sm.remove_session(&peer_id).await;
        assert!(!sm.has_session(&peer_id).await);
        assert_eq!(sm.session_count().await, 0);
    }
}
