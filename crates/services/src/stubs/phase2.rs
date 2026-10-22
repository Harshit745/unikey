use async_trait::async_trait;
use uuid::Uuid;

use anyhow::Result;
use protocol::MessageType;

use crate::phase::Phase;
use crate::traits::{Service, ServiceContext};

/// Stub for Phase 2: Virtual drive mounting and cross-device file browsing.
#[derive(Debug)]
pub struct StubFileExplorer;

#[async_trait]
impl Service for StubFileExplorer {
    fn name(&self) -> &'static str {
        "file_explorer"
    }

    fn phase(&self) -> Phase {
        Phase::Phase2
    }

    async fn start(&self, _ctx: ServiceContext) -> Result<()> {
        tracing::info!("[STUB] FileExplorerService — Phase 2 stub loaded (virtual drive mounting)");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    async fn handle_packet(&self, _peer_id: &Uuid, _msg_type: MessageType, _payload: &[u8]) -> Result<()> {
        Ok(())
    }
}

/// Stub for Phase 2: Shared clipboard search and cross-device search overlay.
#[derive(Debug)]
pub struct StubSearchService;

#[async_trait]
impl Service for StubSearchService {
    fn name(&self) -> &'static str {
        "search"
    }

    fn phase(&self) -> Phase {
        Phase::Phase2
    }

    async fn start(&self, _ctx: ServiceContext) -> Result<()> {
        tracing::info!("[STUB] SearchService — Phase 2 stub loaded (universal search overlay)");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    async fn handle_packet(&self, _peer_id: &Uuid, _msg_type: MessageType, _payload: &[u8]) -> Result<()> {
        Ok(())
    }
}

/// Stub for Phase 2: Shared downloads folder and camera roll auto-sync.
#[derive(Debug)]
pub struct StubAssetSync;

#[async_trait]
impl Service for StubAssetSync {
    fn name(&self) -> &'static str {
        "asset_sync"
    }

    fn phase(&self) -> Phase {
        Phase::Phase2
    }

    async fn start(&self, _ctx: ServiceContext) -> Result<()> {
        tracing::info!("[STUB] AssetSync — Phase 2 stub loaded (shared downloads / camera roll)");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    async fn handle_packet(&self, _peer_id: &Uuid, _msg_type: MessageType, _payload: &[u8]) -> Result<()> {
        Ok(())
    }
}
