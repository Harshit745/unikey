use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use protocol::MessageType;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::traits::{Service, ServiceContext};

/// Lifecycle status of a registered service.
///
/// Updated as the daemon registers, starts, and stops each service so the
/// HTTP API can report a truthful snapshot of the running system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    /// Registered with the registry but not yet started.
    Registered,
    /// `start()` succeeded and the service is active.
    Running,
    /// `stop()` ran — the service has been torn down.
    Stopped,
}

impl ServiceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ServiceStatus::Registered => "registered",
            ServiceStatus::Running => "running",
            ServiceStatus::Stopped => "stopped",
        }
    }
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Thread-safe registry of all feature services.
///
/// Services are registered by name before the daemon starts.
/// The daemon uses this to start/stop services and to route incoming packets.
#[derive(Debug, Clone)]
pub struct ServiceRegistry {
    services: Arc<RwLock<HashMap<&'static str, Box<dyn Service>>>>,
    status: Arc<RwLock<HashMap<&'static str, ServiceStatus>>>,
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            status: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a service by moving it into the registry.
    pub async fn register(&self, service: Box<dyn Service>) {
        let name = service.name();
        let phase = service.phase();
        self.services.write().await.insert(name, service);
        self.status.write().await.insert(name, ServiceStatus::Registered);
        info!("[REGISTRY] Registered service: {name:20} | {phase}");
    }

    /// Start all registered services, each with the provided context.
    pub async fn start_all(&self, ctx: ServiceContext) -> Result<()> {
        let services = self.services.read().await;
        for (name, service) in services.iter() {
            info!("[REGISTRY] Starting service: {name}");
            service.start(ctx.clone()).await?;
            self.status.write().await.insert(*name, ServiceStatus::Running);
        }
        info!("[REGISTRY] All services started successfully");
        Ok(())
    }

    /// Stop all registered services gracefully.
    pub async fn stop_all(&self) -> Result<()> {
        let services = self.services.read().await;
        for (name, service) in services.iter() {
            info!("[REGISTRY] Stopping service: {name}");
            service.stop().await?;
            self.status.write().await.insert(*name, ServiceStatus::Stopped);
        }
        info!("[REGISTRY] All services stopped");
        Ok(())
    }

    /// Route an incoming packet to the correct service based on message type.
    pub async fn route_packet(
        &self,
        peer_id: &Uuid,
        msg_type: MessageType,
        payload: &[u8],
    ) -> Result<()> {
        let service_name = match msg_type {
            MessageType::PairRequest | MessageType::PairResponse => "pairing",
            MessageType::Clipboard => "clipboard",
            MessageType::FileMetadata | MessageType::FileChunk | MessageType::FileAck | MessageType::FileCancel => "files",
            MessageType::InputKeyEvent | MessageType::InputMouseEvent | MessageType::InputGrab | MessageType::InputRelease => "input",
            MessageType::Ping | MessageType::Pong => {
                // Ping/pong handled at transport layer; ignore here
                return Ok(());
            }
            MessageType::Hello | MessageType::Disconnect => {
                // Handled at connection layer
                return Ok(());
            }
        };

        let services = self.services.read().await;
        if let Some(service) = services.get(service_name) {
            service.handle_packet(peer_id, msg_type, payload).await?;
        } else {
            warn!(
                "[REGISTRY] No service registered for {:?} (expected '{service_name}')",
                msg_type
            );
        }
        Ok(())
    }

    /// List all registered services (for logging / health checks).
    pub async fn list_services(&self) -> Vec<(String, String)> {
        let services = self.services.read().await;
        services
            .iter()
            .map(|(name, svc)| (name.to_string(), svc.phase().to_string()))
            .collect()
    }

    /// List all registered services with their lifecycle status.
    ///
    /// Returns `(name, phase, status)` tuples so callers (e.g. the HTTP API)
    /// can report a truthful snapshot of the running system.
    pub async fn list_services_with_status(&self) -> Vec<(String, String, String)> {
        let services = self.services.read().await;
        let status = self.status.read().await;
        services
            .iter()
            .map(|(name, svc)| {
                let st = status
                    .get(name)
                    .copied()
                    .unwrap_or(ServiceStatus::Registered);
                (name.to_string(), svc.phase().to_string(), st.as_str().to_string())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::phase::Phase;
    use crate::traits::{Service, ServiceContext};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    /// Shared, cloneable recording state so a test can inspect what a service
    /// received after handing it to the registry.
    #[derive(Debug, Default, Clone)]
    struct RecorderInner {
        starts: Arc<AtomicUsize>,
        stops: Arc<AtomicUsize>,
        packets: Arc<Mutex<Vec<MessageType>>>,
    }

    /// A recording test service: captures lifecycle + packet events.
    #[derive(Debug, Clone)]
    struct Recorder {
        /// Name as exposed to the registry — must match a routing slot to
        /// receive packets (e.g. "clipboard", "files", "input", "pairing").
        name: &'static str,
        inner: RecorderInner,
    }

    impl Recorder {
        fn named(name: &'static str) -> Self {
            Self {
                name,
                inner: RecorderInner::default(),
            }
        }

        fn packets(&self) -> Vec<MessageType> {
            self.inner.packets.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl Service for Recorder {
        fn name(&self) -> &'static str {
            self.name
        }
        fn phase(&self) -> Phase {
            Phase::Phase1
        }
        async fn start(&self, _ctx: ServiceContext) -> Result<()> {
            self.inner.starts.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        async fn stop(&self) -> Result<()> {
            self.inner.stops.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        async fn handle_packet(
            &self,
            _peer: &Uuid,
            msg_type: MessageType,
            _payload: &[u8],
        ) -> Result<()> {
            self.inner.packets.lock().unwrap().push(msg_type);
            Ok(())
        }
    }

    fn test_context() -> ServiceContext {
        ServiceContext::new(
            device::DeviceIdentity {
                id: Uuid::new_v4(),
                name: "test".into(),
                platform: device::Platform::Linux,
                device_type: device::DeviceType::Desktop,
                created_at: chrono::Utc::now(),
                protocol_version: 1,
                public_key_path: String::new(),
                private_key_path: String::new(),
            },
            transport::session::SessionManager::new(),
            tokio_util::sync::CancellationToken::new(),
        )
    }

    #[tokio::test]
    async fn register_and_list_round_trips() {
        let reg = ServiceRegistry::new();
        reg.register(Box::new(Recorder::named("clipboard"))).await;
        let listed = reg.list_services().await;
        assert!(listed.iter().any(|(name, _)| name == "clipboard"));
    }

    #[tokio::test]
    async fn lifecycle_tracks_status_transitions() {
        let reg = ServiceRegistry::new();
        reg.register(Box::new(Recorder::named("clipboard"))).await;

        let status = reg.list_services_with_status().await;
        assert!(status.iter().any(|(n, _p, s)| n == "clipboard" && s == "registered"));

        reg.start_all(test_context()).await.expect("start");
        let status = reg.list_services_with_status().await;
        assert!(status.iter().any(|(n, _p, s)| n == "clipboard" && s == "running"));

        reg.stop_all().await.expect("stop");
        let status = reg.list_services_with_status().await;
        assert!(status.iter().any(|(n, _p, s)| n == "clipboard" && s == "stopped"));
    }

    #[tokio::test]
    async fn routes_packets_to_the_matching_service() {
        let reg = ServiceRegistry::new();
        let recorder = Recorder::named("clipboard");
        reg.register(Box::new(recorder.clone())).await;

        let peer = Uuid::new_v4();
        reg.route_packet(&peer, MessageType::Clipboard, b"{}").await.unwrap();
        // Transport-owned types are intentionally swallowed, not routed.
        reg.route_packet(&peer, MessageType::Ping, b"").await.unwrap();
        reg.route_packet(&peer, MessageType::Hello, b"").await.unwrap();
        // A different slot's message does not reach the clipboard service.
        reg.route_packet(&peer, MessageType::FileMetadata, b"").await.unwrap();

        // Only the Clipboard message must have reached the clipboard service.
        assert_eq!(recorder.packets(), vec![MessageType::Clipboard]);
    }

    #[tokio::test]
    async fn routing_without_a_service_is_an_ok_noop() {
        let reg = ServiceRegistry::new(); // nothing registered
        let peer = Uuid::new_v4();
        // A message with no owning service must not panic or error.
        assert!(reg.route_packet(&peer, MessageType::FileMetadata, b"").await.is_ok());
        // Ping/Pong/Hello/Disconnect short-circuit at the transport layer.
        assert!(reg.route_packet(&peer, MessageType::Pong, b"").await.is_ok());
        assert!(reg.route_packet(&peer, MessageType::Disconnect, b"").await.is_ok());
    }

    #[tokio::test]
    async fn unregistered_service_falls_back_to_warning_not_error() {
        let reg = ServiceRegistry::new();
        reg.register(Box::new(Recorder::named("recorder"))).await;
        // "files" has no service registered here — routing still succeeds.
        let peer = Uuid::new_v4();
        assert!(reg.route_packet(&peer, MessageType::FileChunk, b"").await.is_ok());
    }
}
