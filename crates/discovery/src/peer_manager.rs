use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::RwLock;
use tokio::time::Instant;
use uuid::Uuid;

use device::DeviceIdentity;

/// Represents the current connection state of a discovered peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerState {
    Online,
    Offline,
    Connecting,
    Connected,
}

/// Holds information about a discovered device on the LAN.
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub identity: DeviceIdentity,
    /// UDP source address the last beacon/reply came from (authoritative IP).
    pub addr: SocketAddr,
    /// TCP port the peer advertises for the device transport.
    pub tcp_port: u16,
    /// Monotonic timestamp of the last packet — used for staleness checks.
    pub last_seen: Instant,
    /// Wall-clock timestamp (epoch millis) of the last packet — for display.
    pub last_seen_unix_ms: u64,
    /// Round-trip time measured via the beacon/reply exchange, if known.
    pub latency_ms: Option<u64>,
    /// When this peer transitioned to `Offline`, if it has. `prune_offline`
    /// removes peers whose `offline_at` is older than the configured grace.
    pub offline_at: Option<Instant>,
    pub state: PeerState,
}

impl PeerInfo {
    /// TCP endpoint advertised by the peer. The UDP source port in `addr` is
    /// ephemeral and must never be used for the transport connection.
    pub fn transport_addr(&self) -> SocketAddr {
        SocketAddr::new(self.addr.ip(), self.tcp_port)
    }
}

/// Manages the lifecycle and state of discovered peers on the network.
#[derive(Debug, Clone)]
pub struct PeerManager {
    peers: Arc<RwLock<HashMap<Uuid, PeerInfo>>>,
}

impl PeerManager {
    /// Creates a new, empty PeerManager.
    pub fn new() -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Adds a new peer if it doesn't exist, or refreshes it if it does.
    ///
    /// Refreshing updates the last-seen timestamps and the peer's address /
    /// advertised TCP port, and brings an `Offline` peer back to `Online`.
    pub async fn add_or_update_peer(
        &self,
        identity: DeviceIdentity,
        addr: SocketAddr,
        tcp_port: u16,
    ) {
        let uuid = identity.id;
        let now = Instant::now();
        let now_unix = now_unix_ms();
        let mut peers = self.peers.write().await;

        if let Some(peer) = peers.get_mut(&uuid) {
            peer.last_seen = now;
            peer.last_seen_unix_ms = now_unix;
            peer.addr = addr;
            peer.tcp_port = tcp_port;

            if peer.state == PeerState::Offline {
                peer.state = PeerState::Online;
                peer.offline_at = None;
            }

            tracing::debug!("[DISCOVERY] Refreshed: {uuid}");
        } else {
            let peer = PeerInfo {
                identity,
                addr,
                tcp_port,
                last_seen: now,
                last_seen_unix_ms: now_unix,
                latency_ms: None,
                offline_at: None,
                state: PeerState::Online,
            };
            peers.insert(uuid, peer);
            tracing::debug!("[DISCOVERY] New device: {uuid}");
        }
    }

    /// Records a fresh round-trip latency measurement for a peer.
    pub async fn record_latency(&self, uuid: &Uuid, latency_ms: u64) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(uuid) {
            peer.latency_ms = Some(latency_ms);
        }
    }

    /// Removes a peer completely from the manager.
    pub async fn remove_peer(&self, uuid: &Uuid) {
        let mut peers = self.peers.write().await;
        peers.remove(uuid);
    }

    /// Returns a snapshot of a specific peer's info if it exists.
    pub async fn get_peer(&self, uuid: &Uuid) -> Option<PeerInfo> {
        let peers = self.peers.read().await;
        peers.get(uuid).cloned()
    }

    /// Returns a snapshot of all currently tracked peers.
    pub async fn get_all(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        peers.values().cloned().collect()
    }

    /// Manually marks a known peer as offline.
    pub async fn mark_offline(&self, uuid: &Uuid) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(uuid)
            && peer.state != PeerState::Offline
        {
            peer.state = PeerState::Offline;
            peer.offline_at = Some(Instant::now());
            tracing::debug!("[DISCOVERY] Offline: {uuid}");
        }
    }

    /// Periodically scans peers and marks those not seen within `timeout` as
    /// Offline. Does not remove them from memory.
    pub async fn cleanup_stale_peers(&self, timeout: Duration) {
        let mut peers = self.peers.write().await;
        let now = Instant::now();

        for (uuid, peer) in peers.iter_mut() {
            if peer.state != PeerState::Offline
                && now.duration_since(peer.last_seen) > timeout
            {
                peer.state = PeerState::Offline;
                peer.offline_at = Some(now);
                tracing::debug!("[DISCOVERY] Offline: {uuid}");
            }
        }
    }

    /// Removes peers that have been Offline for longer than `grace` so a dead
    /// device eventually disappears from the registry (and the GUI).
    pub async fn prune_offline(&self, grace: Duration) {
        let mut peers = self.peers.write().await;
        let before = peers.len();
        peers.retain(|_, peer| {
            !matches!(peer.offline_at, Some(t) if t.elapsed() > grace)
        });
        let pruned = before - peers.len();
        if pruned > 0 {
            tracing::debug!("[DISCOVERY] Pruned {pruned} stale offline peer(s)");
        }
    }
}

impl Default for PeerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Current wall clock in epoch milliseconds.
fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use device::{DeviceType, Platform};

    fn identity(id: Uuid, name: &str) -> DeviceIdentity {
        DeviceIdentity {
            id,
            name: name.to_string(),
            platform: Platform::MacOS,
            device_type: DeviceType::Desktop,
            created_at: chrono::Utc::now(),
            protocol_version: 1,
            public_key_path: String::new(),
            private_key_path: String::new(),
        }
    }

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::from(([192, 168, 1, 10], port))
    }

    #[tokio::test]
    async fn adds_new_peer_online() {
        let pm = PeerManager::new();
        let id = Uuid::new_v4();
        pm.add_or_update_peer(identity(id, "mac"), addr(40000), 45892).await;

        let peers = pm.get_all().await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].state, PeerState::Online);
        assert_eq!(peers[0].tcp_port, 45892);
        assert_eq!(peers[0].transport_addr(), addr(45892));
        assert_eq!(peers[0].identity.name, "mac");
    }

    #[tokio::test]
    async fn refresh_updates_address_port_and_revives_offline() {
        let pm = PeerManager::new();
        let id = Uuid::new_v4();
        pm.add_or_update_peer(identity(id, "mac"), addr(40000), 45892).await;
        pm.mark_offline(&id).await;
        assert_eq!(pm.get_peer(&id).await.unwrap().state, PeerState::Offline);

        pm.add_or_update_peer(identity(id, "mac"), addr(41000), 45900).await;
        let peer = pm.get_peer(&id).await.unwrap();
        assert_eq!(peer.state, PeerState::Online);
        assert_eq!(peer.addr.port(), 41000);
        assert_eq!(peer.tcp_port, 45900);
        assert_eq!(peer.offline_at, None);
    }

    #[tokio::test]
    async fn record_latency_stores_measurement() {
        let pm = PeerManager::new();
        let id = Uuid::new_v4();
        pm.add_or_update_peer(identity(id, "win"), addr(42000), 45892).await;
        pm.record_latency(&id, 4).await;
        assert_eq!(pm.get_peer(&id).await.unwrap().latency_ms, Some(4));
    }

    #[tokio::test]
    async fn cleanup_marks_stale_offline_then_prune_removes() {
        let pm = PeerManager::new();
        let stale = Uuid::new_v4();
        let fresh = Uuid::new_v4();
        pm.add_or_update_peer(identity(stale, "stale"), addr(43000), 45892).await;
        pm.add_or_update_peer(identity(fresh, "fresh"), addr(44000), 45892).await;

        // Age the stale peer past the threshold.
        {
            let mut peers = pm.peers.write().await;
            peers.get_mut(&stale).unwrap().last_seen =
                Instant::now() - Duration::from_secs(60);
        }

        pm.cleanup_stale_peers(Duration::from_secs(15)).await;
        let peers = pm.get_all().await;
        assert_eq!(peers.len(), 2);
        assert_eq!(peers.iter().find(|p| p.identity.id == stale).unwrap().state, PeerState::Offline);
        assert_eq!(peers.iter().find(|p| p.identity.id == fresh).unwrap().state, PeerState::Online);

        // Age the offline marker so the grace period has elapsed.
        {
            let mut peers = pm.peers.write().await;
            peers.get_mut(&stale).unwrap().offline_at =
                Some(Instant::now() - Duration::from_secs(120));
        }

        pm.prune_offline(Duration::from_secs(60)).await;
        let ids: Vec<Uuid> = pm.get_all().await.into_iter().map(|p| p.identity.id).collect();
        assert!(!ids.contains(&stale), "stale peer should be pruned");
        assert!(ids.contains(&fresh), "online peer must survive");
    }

    #[tokio::test]
    async fn prune_keeps_recently_offline_peers() {
        let pm = PeerManager::new();
        let id = Uuid::new_v4();
        pm.add_or_update_peer(identity(id, "just-went-off"), addr(45000), 45892).await;
        pm.mark_offline(&id).await;
        // offline_at is ~now, well within the grace — must be kept.
        pm.prune_offline(Duration::from_secs(60)).await;
        assert_eq!(pm.get_all().await.len(), 1);
    }
}
