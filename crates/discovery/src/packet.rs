use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// UDP beacon broadcast on the LAN to advertise this device.
///
/// Two roles in one packet type:
///   * **Beacon** — a periodic broadcast (`reply_to_sent_at_ms == None`).
///     Receivers record the peer and immediately answer with a unicast reply.
///   * **Reply** — a unicast echo sent immediately upon receiving a beacon
///     (`reply_to_sent_at_ms == Some(their sent_at_ms)`). The original sender
///     computes RTT as `now - reply_to_sent_at_ms` using *its own* clock, so
///     cross-machine clock skew never affects the latency figure.
///
/// The new timing fields carry `#[serde(default)]` so packets from older
/// instances on the same LAN still parse (they simply report no latency).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryPacket {
    pub protocol_version: u32,
    pub device_id: Uuid,
    pub device_name: String,
    pub platform: String,
    pub tcp_port: u16,
    /// Sender wall clock (epoch millis) at the moment this packet was sent.
    #[serde(default)]
    pub sent_at_ms: u64,
    /// `Some(t)` marks this packet as an immediate reply echoing the beacon
    /// that triggered it (whose sender stamped `t`).
    #[serde(default)]
    pub reply_to_sent_at_ms: Option<u64>,
}

impl DiscoveryPacket {
    /// Build a fresh periodic beacon for broadcast.
    #[allow(clippy::too_many_arguments)]
    pub fn beacon(
        protocol_version: u32,
        device_id: Uuid,
        device_name: impl Into<String>,
        platform: impl Into<String>,
        tcp_port: u16,
        sent_at_ms: u64,
    ) -> Self {
        Self {
            protocol_version,
            device_id,
            device_name: device_name.into(),
            platform: platform.into(),
            tcp_port,
            sent_at_ms,
            reply_to_sent_at_ms: None,
        }
    }

    /// Build the unicast reply to `other`, echoing its `sent_at_ms` so the
    /// original sender can measure round-trip time on its own clock.
    pub fn reply_to(&self, other: &Self, sent_at_ms: u64) -> Self {
        Self {
            protocol_version: self.protocol_version,
            device_id: self.device_id,
            device_name: self.device_name.clone(),
            platform: self.platform.clone(),
            tcp_port: self.tcp_port,
            sent_at_ms,
            reply_to_sent_at_ms: Some(other.sent_at_ms),
        }
    }

    /// `true` when this packet is an immediate reply rather than a beacon.
    pub fn is_reply(&self) -> bool {
        self.reply_to_sent_at_ms.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beacon_has_no_reply_target() {
        let b = DiscoveryPacket::beacon(1, Uuid::new_v4(), "mac", "MacOS", 45892, 1000);
        assert!(!b.is_reply());
        assert_eq!(b.reply_to_sent_at_ms, None);
    }

    #[test]
    fn reply_echoes_the_triggering_sent_at() {
        let beacon = DiscoveryPacket::beacon(1, Uuid::new_v4(), "win", "Windows", 45892, 1234);
        let reply = beacon.reply_to(&beacon, 5678);
        assert!(reply.is_reply());
        assert_eq!(reply.reply_to_sent_at_ms, Some(1234));
        assert_eq!(reply.sent_at_ms, 5678);
    }

    #[test]
    fn json_round_trips_new_fields() {
        let b = DiscoveryPacket::beacon(1, Uuid::new_v4(), "linux", "Linux", 45892, 999);
        let json = serde_json::to_string(&b).expect("serialize");
        let parsed: DiscoveryPacket = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.sent_at_ms, 999);
        assert_eq!(parsed.reply_to_sent_at_ms, None);
        assert_eq!(parsed.device_name, "linux");
    }

    #[test]
    fn old_style_packet_without_timing_fields_still_parses() {
        // A packet as sent by a pre-latency daemon — new fields must default.
        let json = r#"{
            "protocol_version": 1,
            "device_id": "00000000-0000-0000-0000-000000000001",
            "device_name": "old",
            "platform": "MacOS",
            "tcp_port": 45892
        }"#;
        let parsed: DiscoveryPacket = serde_json::from_str(json).expect("lenient parse");
        assert_eq!(parsed.sent_at_ms, 0);
        assert_eq!(parsed.reply_to_sent_at_ms, None);
        assert!(!parsed.is_reply());
    }
}
