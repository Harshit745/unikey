use async_trait::async_trait;
use uuid::Uuid;

use anyhow::Result;
use protocol::MessageType;

use crate::phase::Phase;
use crate::traits::{Service, ServiceContext};

/// Stub for Phase 5: AI agent aware of device resource metrics (CPU, RAM, battery, network).
#[derive(Debug)]
pub struct StubAiContext;

#[async_trait]
impl Service for StubAiContext {
    fn name(&self) -> &'static str {
        "ai_context"
    }

    fn phase(&self) -> Phase {
        Phase::Phase5
    }

    async fn start(&self, _ctx: ServiceContext) -> Result<()> {
        tracing::info!("[STUB] AiContext — Phase 5 stub loaded (device intelligence / workload routing)");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    async fn handle_packet(&self, _peer_id: &Uuid, _msg_type: MessageType, _payload: &[u8]) -> Result<()> {
        Ok(())
    }
}

/// Stub for Phase 5: Natural language clipboard search and auto-categorization.
#[derive(Debug)]
pub struct StubAiClipboard;

#[async_trait]
impl Service for StubAiClipboard {
    fn name(&self) -> &'static str {
        "ai_clipboard"
    }

    fn phase(&self) -> Phase {
        Phase::Phase5
    }

    async fn start(&self, _ctx: ServiceContext) -> Result<()> {
        tracing::info!("[STUB] AiClipboard — Phase 5 stub loaded (NL search / auto-summarize)");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    async fn handle_packet(&self, _peer_id: &Uuid, _msg_type: MessageType, _payload: &[u8]) -> Result<()> {
        Ok(())
    }
}

/// Stub for Phase 5: Voice commands and trigger-based automation rules.
#[derive(Debug)]
pub struct StubAutomation;

#[async_trait]
impl Service for StubAutomation {
    fn name(&self) -> &'static str {
        "automation"
    }

    fn phase(&self) -> Phase {
        Phase::Phase5
    }

    async fn start(&self, _ctx: ServiceContext) -> Result<()> {
        tracing::info!("[STUB] Automation — Phase 5 stub loaded (voice commands / trigger rules)");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    async fn handle_packet(&self, _peer_id: &Uuid, _msg_type: MessageType, _payload: &[u8]) -> Result<()> {
        Ok(())
    }
}
