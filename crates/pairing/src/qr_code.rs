use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Encodes the data needed to render a QR code for out-of-band pairing.
///
/// The Flutter overlay reads this, renders it as a QR code,
/// and the peer scans it to establish initial trust without PIN entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrPairingPayload {
    /// UUID of the device offering the QR code.
    pub device_id: Uuid,
    /// Human-readable device name.
    pub device_name: String,
    /// IP address or hostname the peer should connect to.
    pub address: String,
    /// TCP port for the transport layer.
    pub tcp_port: u16,
    /// Ephemeral symmetric key for the initial encrypted handshake.
    pub session_key: [u8; 32],
}

impl QrPairingPayload {
    /// Serialize to JSON bytes for transmission.
    pub fn encode(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserialize from JSON bytes.
    pub fn decode(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}
