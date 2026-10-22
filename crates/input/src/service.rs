use async_trait::async_trait;
use uuid::Uuid;

use anyhow::Result;
use protocol::MessageType;
use services::{Phase, Service, ServiceContext};

use crate::event::{InputAction, KeyEvent, MouseEvent};
use crate::platform::InputInjector;

/// The Input Service forwards keyboard and mouse events between connected devices.
///
/// This enables:
/// - Edge switching (mouse warping across screen boundaries between devices).
/// - Global hotkey switching (Ctrl+Alt+Arrow to jump focus between machines).
/// - Low-latency keyboard/mouse event forwarding for unified input.
#[derive(Debug)]
pub struct InputService {
    injector: Box<dyn InputInjector>,
}

impl InputService {
    pub fn new() -> Self {
        Self {
            injector: crate::platform::create_input_injector(),
        }
    }
}

impl Default for InputService {
    fn default() -> Self {
        Self::new()
    }
}

impl InputService {
    /// Forward a keyboard event to a connected peer via the session manager.
    pub async fn forward_keyboard(
        &self,
        peer_id: &Uuid,
        event: KeyEvent,
        session_manager: &transport::session::SessionManager,
    ) -> Result<()> {
        let payload = serde_json::to_vec(&InputAction::Keyboard(event))?;
        let packet = protocol::Packet {
            version: 1,
            message_type: MessageType::InputKeyEvent,
            payload,
        };

        if let Some(tx) = session_manager.get_session_sender(peer_id).await {
            tx.send(packet).await?;
        }
        Ok(())
    }

    /// Forward a mouse event to a connected peer via the session manager.
    pub async fn forward_mouse(
        &self,
        peer_id: &Uuid,
        event: MouseEvent,
        session_manager: &transport::session::SessionManager,
    ) -> Result<()> {
        let payload = serde_json::to_vec(&InputAction::Mouse(event))?;
        let packet = protocol::Packet {
            version: 1,
            message_type: MessageType::InputMouseEvent,
            payload,
        };

        if let Some(tx) = session_manager.get_session_sender(peer_id).await {
            tx.send(packet).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl Service for InputService {
    fn name(&self) -> &'static str {
        "input"
    }

    fn phase(&self) -> Phase {
        Phase::Phase1
    }

    async fn start(&self, _ctx: ServiceContext) -> Result<()> {
        tracing::info!("[INPUT] Service started");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        tracing::info!("[INPUT] Service stopped");
        Ok(())
    }

    async fn handle_packet(
        &self,
        peer_id: &Uuid,
        msg_type: MessageType,
        payload: &[u8],
    ) -> Result<()> {
        match msg_type {
            MessageType::InputKeyEvent | MessageType::InputMouseEvent => {
                let action: InputAction = serde_json::from_slice(payload)?;
                tracing::debug!("[INPUT] Received from {peer_id}: {action:?}");
                self.injector.inject(&action)?;
            }
            _ => {
                tracing::debug!("[INPUT] Unhandled packet type from {peer_id}: {msg_type:?}");
            }
        }
        Ok(())
    }
}
