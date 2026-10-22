use async_trait::async_trait;
use std::fmt::Debug;
use uuid::Uuid;

use anyhow::Result;
use protocol::MessageType;
use transport::session::SessionManager;
use device::DeviceIdentity;
use events::EventBus;

use crate::phase::Phase;

/// Context provided to every service at startup.
/// Contains the local device identity, references to core subsystems,
/// and a cancellation token for graceful shutdown.
#[derive(Clone, Debug)]
pub struct ServiceContext {
    /// Identity of the local device running this daemon.
    pub local_device: DeviceIdentity,
    /// Manager for all active peer sessions — used to send packets to connected peers.
    pub session_manager: SessionManager,
    /// Token signalled when the daemon is shutting down.
    /// Services should watch this and terminate cleanly.
    pub shutdown: tokio_util::sync::CancellationToken,
    /// Pub/sub bus for emitting live events to the GUI (SSE).
    pub events: EventBus,
}

impl ServiceContext {
    pub fn new(
        local_device: DeviceIdentity,
        session_manager: SessionManager,
        shutdown: tokio_util::sync::CancellationToken,
    ) -> Self {
        Self::with_events(
            local_device,
            session_manager,
            shutdown,
            EventBus::new(),
        )
    }

    pub fn with_events(
        local_device: DeviceIdentity,
        session_manager: SessionManager,
        shutdown: tokio_util::sync::CancellationToken,
        events: EventBus,
    ) -> Self {
        Self {
            local_device,
            session_manager,
            shutdown,
            events,
        }
    }
}

/// Every feature module in UniKey implements this trait.
///
/// Services are registered in the [`ServiceRegistry`](crate::registry::ServiceRegistry)
/// and started/stopped by the daemon lifecycle manager.
#[async_trait]
pub trait Service: Send + Sync + Debug {
    /// Human-readable name for this service (e.g. "clipboard", "pairing").
    fn name(&self) -> &'static str;

    /// Which roadmap phase this service belongs to.
    fn phase(&self) -> Phase;

    /// Called once during daemon startup after the transport layer is ready.
    /// The service should spawn any background tasks and register packet handlers.
    async fn start(&self, ctx: ServiceContext) -> Result<()>;

    /// Called during graceful daemon shutdown.
    /// The service should flush state, save pending data, and terminate background tasks.
    async fn stop(&self) -> Result<()>;

    /// Called when a packet arrives from a connected peer.
    /// The daemon's packet router dispatches to the correct service based on `msg_type`.
    async fn handle_packet(
        &self,
        peer_id: &Uuid,
        msg_type: MessageType,
        payload: &[u8],
    ) -> Result<()>;
}
