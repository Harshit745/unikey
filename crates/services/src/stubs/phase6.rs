use async_trait::async_trait;
use uuid::Uuid;

use anyhow::Result;
use protocol::MessageType;

use crate::phase::Phase;
use crate::traits::{Service, ServiceContext};

/// Stub for Phase 6: Central policy engine and remote fleet management.
#[derive(Debug)]
pub struct StubAdmin;

#[async_trait]
impl Service for StubAdmin {
    fn name(&self) -> &'static str {
        "admin"
    }

    fn phase(&self) -> Phase {
        Phase::Phase6
    }

    async fn start(&self, _ctx: ServiceContext) -> Result<()> {
        tracing::info!("[STUB] Admin — Phase 6 stub loaded (policy engine / SSO / audit)");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    async fn handle_packet(&self, _peer_id: &Uuid, _msg_type: MessageType, _payload: &[u8]) -> Result<()> {
        Ok(())
    }
}

/// Stub for Phase 6: Fleet management console and remote deployment.
#[derive(Debug)]
pub struct StubFleet;

#[async_trait]
impl Service for StubFleet {
    fn name(&self) -> &'static str {
        "fleet"
    }

    fn phase(&self) -> Phase {
        Phase::Phase6
    }

    async fn start(&self, _ctx: ServiceContext) -> Result<()> {
        tracing::info!("[STUB] Fleet — Phase 6 stub loaded (fleet console / remote deploy)");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    async fn handle_packet(&self, _peer_id: &Uuid, _msg_type: MessageType, _payload: &[u8]) -> Result<()> {
        Ok(())
    }
}
