use async_trait::async_trait;
use uuid::Uuid;

use anyhow::Result;
use protocol::MessageType;

use crate::phase::Phase;
use crate::traits::{Service, ServiceContext};

/// Stub for Phase 4: Multi-user cursor sharing and collaborative workspace.
#[derive(Debug)]
pub struct StubCollaboration;

#[async_trait]
impl Service for StubCollaboration {
    fn name(&self) -> &'static str {
        "collaboration"
    }

    fn phase(&self) -> Phase {
        Phase::Phase4
    }

    async fn start(&self, _ctx: ServiceContext) -> Result<()> {
        tracing::info!("[STUB] Collaboration — Phase 4 stub loaded (multi-user cursors / whiteboard)");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    async fn handle_packet(&self, _peer_id: &Uuid, _msg_type: MessageType, _payload: &[u8]) -> Result<()> {
        Ok(())
    }
}

/// Stub for Phase 4: Team workspace management with guest access.
#[derive(Debug)]
pub struct StubWorkspace;

#[async_trait]
impl Service for StubWorkspace {
    fn name(&self) -> &'static str {
        "workspace"
    }

    fn phase(&self) -> Phase {
        Phase::Phase4
    }

    async fn start(&self, _ctx: ServiceContext) -> Result<()> {
        tracing::info!("[STUB] Workspace — Phase 4 stub loaded (team workspaces / guest access)");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    async fn handle_packet(&self, _peer_id: &Uuid, _msg_type: MessageType, _payload: &[u8]) -> Result<()> {
        Ok(())
    }
}
