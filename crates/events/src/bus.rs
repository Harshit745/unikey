use std::collections::VecDeque;
use std::sync::Arc;

use tokio::sync::{RwLock, broadcast};

use crate::event::{AppEvent, Envelope};

/// How many events are kept in the in-memory history for late subscribers.
const HISTORY_LIMIT: usize = 200;

/// Broadcast channel capacity. If a subscriber falls this far behind, the
/// oldest unread events are dropped for it (broadcast semantics) — the GUI is
/// expected to re-pull full state on reconnect.
const CHANNEL_CAPACITY: usize = 512;

/// Pub/sub event bus shared by the daemon core, services, and the HTTP API.
///
/// `publish` fans an event out to every live SSE subscriber and appends it to
/// a bounded history so a subscriber that connects after an event still sees
/// recent activity.
#[derive(Debug, Clone)]
pub struct EventBus {
    tx: broadcast::Sender<Envelope>,
    history: Arc<RwLock<VecDeque<Envelope>>>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            tx,
            history: Arc::new(RwLock::new(VecDeque::with_capacity(HISTORY_LIMIT))),
        }
    }

    /// Publish an event to all current subscribers and the history buffer.
    ///
    /// This is synchronous and never blocks: if a slow subscriber overruns the
    /// channel, the broadcast drops the oldest unread event for it rather than
    /// stalling the publisher.
    pub fn publish(&self, event: AppEvent) {
        let envelope = Envelope::new(event);
        // Best effort — with no subscribers the returned receiver is dropped
        // and `send` just discards.
        let _ = self.tx.send(envelope.clone());

        let history = self.history.clone();
        let mut guard = match history.try_write() {
            Ok(g) => g,
            Err(_) => return, // reader holding the lock — history write is best effort
        };
        guard.push_back(envelope);
        while guard.len() > HISTORY_LIMIT {
            guard.pop_front();
        }
    }

    /// Subscribe to the live event stream. Events published after this call
    /// are delivered; use [`history`](Self::history) for recent past events.
    pub fn subscribe(&self) -> broadcast::Receiver<Envelope> {
        self.tx.subscribe()
    }

    /// The most recent events, newest first.
    pub async fn history(&self, limit: usize) -> Vec<Envelope> {
        let guard = self.history.read().await;
        guard.iter().rev().take(limit.clamp(1, HISTORY_LIMIT)).cloned().collect()
    }

    /// How many live subscribers are currently connected.
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn publishes_to_live_subscribers() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let peer_id = Uuid::new_v4();

        bus.publish(AppEvent::DeviceAdded {
            peer_id,
            peer_name: "bob".into(),
        });

        let env = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            rx.recv(),
        )
        .await
        .expect("event within timeout")
        .expect("channel open");
        match env.event {
            AppEvent::DeviceAdded { peer_id: got, peer_name } => {
                assert_eq!(got, peer_id);
                assert_eq!(peer_name, "bob");
            }
            other => panic!("unexpected event {other:?}"),
        }
    }

    #[tokio::test]
    async fn history_is_bounded_and_newest_first() {
        let bus = EventBus::new();
        for i in 0..300u32 {
            bus.publish(AppEvent::ServiceStatus {
                name: "svc".into(),
                status: format!("v{i}"),
            });
        }
        let history = bus.history(usize::MAX).await;
        assert_eq!(history.len(), HISTORY_LIMIT);
        // Newest event is v299.
        let newest = &history[0].event;
        match newest {
            AppEvent::ServiceStatus { status, .. } => assert_eq!(status, "v299"),
            other => panic!("unexpected {other:?}"),
        }
    }
}
