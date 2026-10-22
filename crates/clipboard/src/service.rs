use std::collections::VecDeque;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use anyhow::Result;
use chrono::{DateTime, Utc};
use events::{AppEvent, EventBus};
use protocol::{MessageType, Packet};
use services::{Phase, Service, ServiceContext};

use crate::platform::{ClipboardAccess, ClipboardContent};

/// How many past clipboard entries to keep in memory (history view).
const HISTORY_LIMIT: usize = 100;

/// A single entry in the clipboard history.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ClipboardHistoryEntry {
    pub direction: &'static str,
    pub peer_id: Option<Uuid>,
    pub content_type: String,
    /// Text preview (first 200 chars) for the UI.
    pub preview: String,
    pub timestamp: DateTime<Utc>,
}

/// The Clipboard Service synchronizes clipboard content across all connected
/// and trusted devices in near-real-time.
///
/// Architecture:
/// - A background task polls the local system clipboard every ~250ms.
/// - Content that differs from what we last knew is broadcast to all sessions.
/// - Incoming clipboard packets are written locally, and echo-suppressed so the
///   poll loop does not bounce them straight back to the sender.
/// - A bounded in-memory history is kept for the API / GUI.
#[derive(Debug)]
pub struct ClipboardService {
    platform: Box<dyn ClipboardAccess>,
    /// The last content this device produced or wrote (echo suppression).
    last_known: Arc<Mutex<Option<ClipboardContent>>>,
    /// In-memory clipboard history (bounded).
    history: Arc<Mutex<VecDeque<ClipboardHistoryEntry>>>,
    /// Live event bus, captured at `start()`.
    events: Arc<RwLock<Option<EventBus>>>,
}

impl ClipboardService {
    pub fn new() -> Self {
        Self::with_platform(crate::platform::create_clipboard_access())
    }

    /// Build a service with an explicit clipboard backend. Production uses
    /// [`Self::new`] (arboard with a stub fallback); tests inject a mock so
    /// the service logic can run without a real system clipboard.
    pub fn with_platform(platform: Box<dyn ClipboardAccess>) -> Self {
        Self {
            platform,
            last_known: Arc::new(Mutex::new(None)),
            history: Arc::new(Mutex::new(VecDeque::with_capacity(HISTORY_LIMIT))),
            events: Arc::new(RwLock::new(None)),
        }
    }

    /// Return a snapshot of recent clipboard history, newest first.
    pub async fn history(&self) -> Vec<ClipboardHistoryEntry> {
        let history = self.history.lock().await;
        history.iter().rev().cloned().collect()
    }

    async fn record_history(
        &self,
        direction: &'static str,
        peer_id: Option<Uuid>,
        content: &ClipboardContent,
    ) {
        let preview = content
            .text
            .clone()
            .or_else(|| content.html.clone())
            .unwrap_or_default();
        let preview: String = preview.chars().take(200).collect();
        let mut history = self.history.lock().await;
        history.push_back(ClipboardHistoryEntry {
            direction,
            peer_id,
            content_type: content.content_type.clone(),
            preview,
            timestamp: Utc::now(),
        });
        while history.len() > HISTORY_LIMIT {
            history.pop_front();
        }
    }
}

impl Default for ClipboardService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Service for ClipboardService {
    fn name(&self) -> &'static str {
        "clipboard"
    }

    fn phase(&self) -> Phase {
        Phase::Phase1
    }

    async fn start(&self, ctx: ServiceContext) -> Result<()> {
        *self.events.write().await = Some(ctx.events.clone());
        let platform = self.platform.clone_box();
        let session_manager = ctx.session_manager;
        let shutdown = ctx.shutdown.clone();
        let last_known = self.last_known.clone();
        let history = self.history.clone();
        let events = self.events.clone();

        // Background sync loop: poll clipboard and broadcast changes.
        tokio::spawn(async move {
            let poll_interval = std::time::Duration::from_millis(250);

            loop {
                tokio::select! {
                    _ = tokio::time::sleep(poll_interval) => {}
                    _ = shutdown.cancelled() => {
                        tracing::info!("[CLIPBOARD] Sync loop stopped");
                        return;
                    }
                }

                let content = match platform.read() {
                    Ok(Some(c)) => c,
                    Ok(None) => continue,
                    Err(e) => {
                        tracing::warn!("[CLIPBOARD] Read error: {e}");
                        continue;
                    }
                };

                // Echo suppression: skip anything we already know about
                // (including what we just wrote on behalf of a remote peer).
                let mut known = last_known.lock().await;
                if known.as_ref() == Some(&content) {
                    continue;
                }
                *known = Some(content.clone());
                drop(known);

                let payload = match serde_json::to_vec(&content) {
                    Ok(b) => b,
                    Err(e) => {
                        tracing::warn!("[CLIPBOARD] Serialization failed: {e}");
                        continue;
                    }
                };

                let packet = Packet {
                    version: 1,
                    message_type: MessageType::Clipboard,
                    payload,
                };

                session_manager.broadcast_packet(packet).await;
                tracing::debug!(
                    "[CLIPBOARD] Synced new content to {} session(s)",
                    session_manager.session_count().await,
                );

                if let Some(bus) = events.read().await.clone() {
                    let preview = content
                        .text
                        .clone()
                        .unwrap_or_default()
                        .chars()
                        .take(200)
                        .collect();
                    bus.publish(AppEvent::ClipboardSynced {
                        direction: "out".to_string(),
                        peer_id: None,
                        content_type: content.content_type.clone(),
                        preview,
                    });
                }

                let mut history = history.lock().await;
                history.push_back(ClipboardHistoryEntry {
                    direction: "out",
                    peer_id: None,
                    content_type: content.content_type.clone(),
                    preview: content
                        .text
                        .unwrap_or_default()
                        .chars()
                        .take(200)
                        .collect(),
                    timestamp: Utc::now(),
                });
                while history.len() > HISTORY_LIMIT {
                    history.pop_front();
                }
            }
        });

        tracing::info!("[CLIPBOARD] Service started");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        tracing::info!("[CLIPBOARD] Service stopped");
        Ok(())
    }

    async fn handle_packet(
        &self,
        peer_id: &Uuid,
        msg_type: MessageType,
        payload: &[u8],
    ) -> Result<()> {
        if msg_type == MessageType::Clipboard {
            let content: ClipboardContent = serde_json::from_slice(payload)?;
            self.platform.write(&content)?;

            // Echo suppression: remember this content so the poll loop treats it
            // as "known" and does not re-broadcast it back to the sender.
            let mut known = self.last_known.lock().await;
            *known = Some(content.clone());
            drop(known);

            self.record_history("in", Some(*peer_id), &content).await;
            tracing::info!("[CLIPBOARD] Received {} from {peer_id}", content.content_type);

            if let Some(bus) = self.events.read().await.clone() {
                let preview = content
                    .text
                    .clone()
                    .unwrap_or_default()
                    .chars()
                    .take(200)
                    .collect();
                bus.publish(AppEvent::ClipboardSynced {
                    direction: "in".to_string(),
                    peer_id: Some(*peer_id),
                    content_type: content.content_type.clone(),
                    preview,
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{ClipboardAccess, ClipboardContent, ImagePayload};

    /// In-memory clipboard backend: holds settable content and records writes.
    #[derive(Debug, Default)]
    struct MockClipboard {
        content: std::sync::Mutex<Option<ClipboardContent>>,
        writes: std::sync::Mutex<Vec<ClipboardContent>>,
    }

    impl MockClipboard {
        fn with_text(text: &str) -> Self {
            Self {
                content: std::sync::Mutex::new(Some(ClipboardContent {
                    content_type: "text/plain".into(),
                    text: Some(text.into()),
                    html: None,
                    image: None,
                })),
                writes: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    impl ClipboardAccess for MockClipboard {
        fn read(&self) -> Result<Option<ClipboardContent>> {
            Ok(self.content.lock().unwrap().clone())
        }
        fn write(&self, content: &ClipboardContent) -> Result<()> {
            *self.content.lock().unwrap() = Some(content.clone());
            self.writes.lock().unwrap().push(content.clone());
            Ok(())
        }
        fn clone_box(&self) -> Box<dyn ClipboardAccess> {
            Box::new(MockClipboard {
                content: self.content.lock().unwrap().clone().into(),
                writes: std::sync::Mutex::new(self.writes.lock().unwrap().clone()),
            })
        }
    }

    fn text_content(text: &str) -> ClipboardContent {
        ClipboardContent {
            content_type: "text/plain".into(),
            text: Some(text.into()),
            html: None,
            image: None,
        }
    }

    #[tokio::test]
    async fn history_is_bounded_and_newest_first() {
        let svc = ClipboardService::with_platform(Box::new(MockClipboard::default()));
        for i in 0..150 {
            svc.record_history("out", None, &text_content(&format!("item {i}"))).await;
        }
        let history = svc.history().await;
        assert_eq!(history.len(), 100, "history must be bounded to 100");
        // Newest first.
        assert_eq!(history[0].preview, "item 149");
        assert_eq!(history[99].preview, "item 50");
        // Entries carry their direction.
        assert_eq!(history[0].direction, "out");
    }

    #[tokio::test]
    async fn preview_falls_back_to_html_and_is_truncated() {
        let svc = ClipboardService::with_platform(Box::new(MockClipboard::default()));
        let long_html = "x".repeat(500);
        let content = ClipboardContent {
            content_type: "text/html".into(),
            text: None,
            html: Some(long_html),
            image: None,
        };
        svc.record_history("in", Some(Uuid::new_v4()), &content).await;
        let entry = svc.history().await[0].clone();
        assert_eq!(entry.content_type, "text/html");
        assert_eq!(entry.preview.len(), 200);
        assert!(entry.peer_id.is_some());
    }

    #[tokio::test]
    async fn image_content_records_a_png_preview() {
        let svc = ClipboardService::with_platform(Box::new(MockClipboard::default()));
        let content = ClipboardContent {
            content_type: "image/png".into(),
            text: None,
            html: None,
            image: Some(ImagePayload {
                width: 2,
                height: 2,
                rgba_base64: "AAAA".into(),
            }),
        };
        svc.record_history("in", None, &content).await;
        let entry = svc.history().await[0].clone();
        assert_eq!(entry.content_type, "image/png");
        assert_eq!(entry.preview, ""); // no text/html fallback for images
    }

    #[tokio::test]
    async fn inbound_packet_writes_to_platform_and_records_history() {
        let mock = MockClipboard::with_text("old");
        let svc = ClipboardService::with_platform(Box::new(mock));
        let peer = Uuid::new_v4();

        let content = text_content("hello from peer");
        let payload = serde_json::to_vec(&content).unwrap();
        svc.handle_packet(&peer, MessageType::Clipboard, &payload).await.unwrap();

        // The peer's content was written to the clipboard: the mock's read()
        // now returns what we handed to handle_packet.
        assert_eq!(
            svc.platform.read().unwrap().as_ref(),
            Some(&content),
            "peer content must be written to the system clipboard"
        );

        // …recorded in history as inbound, and echo-suppressed.
        let entry = svc.history().await[0].clone();
        assert_eq!(entry.direction, "in");
        assert_eq!(entry.peer_id, Some(peer));
        assert_eq!(entry.preview, "hello from peer");

        let known = svc.last_known.lock().await;
        assert_eq!(known.as_ref(), Some(&content), "last_known must suppress echo");
    }

    #[tokio::test]
    async fn non_clipboard_packets_are_ignored() {
        let svc = ClipboardService::with_platform(Box::new(MockClipboard::with_text("keep")));
        let peer = Uuid::new_v4();
        svc.handle_packet(&peer, MessageType::FileMetadata, b"[]").await.unwrap();
        assert!(svc.history().await.is_empty(), "no history for foreign messages");
        assert_eq!(
            svc.platform.read().unwrap().as_ref().unwrap().text.as_deref(),
            Some("keep"),
            "clipboard must be untouched"
        );
    }

    #[tokio::test]
    async fn malformed_clipboard_payload_is_an_error_not_a_panic() {
        let svc = ClipboardService::with_platform(Box::new(MockClipboard::default()));
        let peer = Uuid::new_v4();
        let err = svc.handle_packet(&peer, MessageType::Clipboard, b"not-json").await;
        assert!(err.is_err());
        assert!(svc.history().await.is_empty());
    }
}
