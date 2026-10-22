use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single event published by a service or the daemon core, consumed by the
/// desktop GUI over the SSE endpoint (`GET /api/v1/events`).
///
/// The enum is flattened into the SSE envelope so each event carries a
/// `type` field (e.g. `"clipboard_synced"`) plus its own fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppEvent {
    /// A service changed lifecycle status (started / stopped / failed).
    ServiceStatus { name: String, status: String },

    /// An encrypted session with a peer was established.
    SessionConnected { peer_id: Uuid, peer_name: String },

    /// An encrypted session was torn down.
    SessionDisconnected { peer_id: Uuid, peer_name: String },

    /// Clipboard content was synced to (out) or received from (in) a peer.
    ClipboardSynced {
        direction: String,
        peer_id: Option<Uuid>,
        content_type: String,
        preview: String,
    },

    /// A file transfer was queued (sender) or accepted (receiver).
    TransferStarted {
        transfer_id: Uuid,
        peer_id: Uuid,
        filename: String,
        total_size: u64,
        direction: String,
    },

    /// Progress on a file transfer (throttled to ~10 Hz in the service).
    TransferProgress {
        transfer_id: Uuid,
        transferred: u64,
        total: u64,
    },

    /// A transfer finished successfully.
    TransferCompleted {
        transfer_id: Uuid,
        filename: String,
        verified: bool,
    },

    /// A transfer failed (I/O, checksum, peer error).
    TransferFailed { transfer_id: Uuid, error: String },

    /// A transfer was cancelled by either side.
    TransferCancelled { transfer_id: Uuid },

    /// A device completed the pairing flow and became trusted.
    PairingCompleted { peer_id: Uuid, peer_name: String },

    /// A device was added to the trust store.
    DeviceAdded { peer_id: Uuid, peer_name: String },

    /// A device was removed from the trust store.
    DeviceRemoved { peer_id: Uuid },
}

/// An event stamped with the time it was published.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub timestamp: DateTime<Utc>,
    #[serde(flatten)]
    pub event: AppEvent,
}

impl Envelope {
    pub fn new(event: AppEvent) -> Self {
        Self {
            timestamp: Utc::now(),
            event,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_with_type_discriminator() {
        let env = Envelope::new(AppEvent::ClipboardSynced {
            direction: "out".into(),
            peer_id: None,
            content_type: "text/plain".into(),
            preview: "hello".into(),
        });
        let json = serde_json::to_value(&env).unwrap();
        assert_eq!(json["type"], "clipboard_synced");
        assert_eq!(json["direction"], "out");
        assert_eq!(json["content_type"], "text/plain");
        assert!(json["timestamp"].is_string());
    }

    #[test]
    fn round_trips_all_variants() {
        let events = vec![
            AppEvent::ServiceStatus {
                name: "clipboard".into(),
                status: "running".into(),
            },
            AppEvent::SessionConnected {
                peer_id: Uuid::new_v4(),
                peer_name: "bob".into(),
            },
            AppEvent::SessionDisconnected {
                peer_id: Uuid::new_v4(),
                peer_name: "bob".into(),
            },
            AppEvent::ClipboardSynced {
                direction: "in".into(),
                peer_id: Some(Uuid::new_v4()),
                content_type: "image/png".into(),
                preview: "".into(),
            },
            AppEvent::TransferStarted {
                transfer_id: Uuid::new_v4(),
                peer_id: Uuid::new_v4(),
                filename: "a.bin".into(),
                total_size: 42,
                direction: "send".into(),
            },
            AppEvent::TransferProgress {
                transfer_id: Uuid::new_v4(),
                transferred: 10,
                total: 42,
            },
            AppEvent::TransferCompleted {
                transfer_id: Uuid::new_v4(),
                filename: "a.bin".into(),
                verified: true,
            },
            AppEvent::TransferFailed {
                transfer_id: Uuid::new_v4(),
                error: "boom".into(),
            },
            AppEvent::TransferCancelled {
                transfer_id: Uuid::new_v4(),
            },
            AppEvent::PairingCompleted {
                peer_id: Uuid::new_v4(),
                peer_name: "carol".into(),
            },
            AppEvent::DeviceAdded {
                peer_id: Uuid::new_v4(),
                peer_name: "dave".into(),
            },
            AppEvent::DeviceRemoved {
                peer_id: Uuid::new_v4(),
            },
        ];
        for e in events {
            let env = Envelope::new(e.clone());
            let json = serde_json::to_vec(&env).unwrap();
            let back: Envelope = serde_json::from_slice(&json).unwrap();
            assert_eq!(back.event, e);
        }
    }
}
