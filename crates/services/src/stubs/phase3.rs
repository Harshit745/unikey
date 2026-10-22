use async_trait::async_trait;
use uuid::Uuid;

use anyhow::Result;
use protocol::MessageType;

use crate::phase::Phase;
use crate::traits::{Service, ServiceContext};

/// Stub for Phase 3: Route audio from one device to another (e.g., desktop → Mac speakers).
#[derive(Debug)]
pub struct StubAudioRouting;

#[async_trait]
impl Service for StubAudioRouting {
    fn name(&self) -> &'static str {
        "audio_routing"
    }

    fn phase(&self) -> Phase {
        Phase::Phase3
    }

    async fn start(&self, _ctx: ServiceContext) -> Result<()> {
        tracing::info!("[STUB] AudioRouting — Phase 3 stub loaded (cross-device audio/mic/webcam)");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    async fn handle_packet(&self, _peer_id: &Uuid, _msg_type: MessageType, _payload: &[u8]) -> Result<()> {
        Ok(())
    }
}

/// Stub for Phase 3: Window and session state teleportation between devices.
#[derive(Debug)]
pub struct StubMediaShare;

#[async_trait]
impl Service for StubMediaShare {
    fn name(&self) -> &'static str {
        "media_share"
    }

    fn phase(&self) -> Phase {
        Phase::Phase3
    }

    async fn start(&self, _ctx: ServiceContext) -> Result<()> {
        tracing::info!("[STUB] MediaShare — Phase 3 stub loaded (window teleport / state migration)");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    async fn handle_packet(&self, _peer_id: &Uuid, _msg_type: MessageType, _payload: &[u8]) -> Result<()> {
        Ok(())
    }
}

/// Stub for Phase 3: Virtual storage pool across multiple device drives.
#[derive(Debug)]
pub struct StubVirtualStorage;

#[async_trait]
impl Service for StubVirtualStorage {
    fn name(&self) -> &'static str {
        "virtual_storage"
    }

    fn phase(&self) -> Phase {
        Phase::Phase3
    }

    async fn start(&self, _ctx: ServiceContext) -> Result<()> {
        tracing::info!("[STUB] VirtualStorage — Phase 3 stub loaded (unified volume pool)");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    async fn handle_packet(&self, _peer_id: &Uuid, _msg_type: MessageType, _payload: &[u8]) -> Result<()> {
        Ok(())
    }
}
