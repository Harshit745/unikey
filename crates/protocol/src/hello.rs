use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// Serialize 32-byte array as lowercase hex string.
fn ser_key32<S: Serializer>(bytes: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
    let hex = bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>();
    s.serialize_str(&hex)
}

/// Deserialize 32-byte array from lowercase hex string (or missing → zeros).
fn de_key32<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
    let opt = Option::<String>::deserialize(d)?;
    match opt {
        Some(s) if s.len() == 64 => {
            let mut out = [0u8; 32];
            for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
                out[i] = u8::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16).unwrap();
            }
            Ok(out)
        }
        _ => Ok([0; 32]),
    }
}

/// Serialize 64-byte array as lowercase hex string.
fn ser_key64<S: Serializer>(bytes: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
    let hex = bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>();
    s.serialize_str(&hex)
}

/// Deserialize 64-byte array from lowercase hex string (or missing → zeros).
fn de_key64<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
    let opt = Option::<String>::deserialize(d)?;
    match opt {
        Some(s) if s.len() == 128 => {
            let mut out = [0u8; 64];
            for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
                out[i] = u8::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16).unwrap();
            }
            Ok(out)
        }
        _ => Ok([0; 64]),
    }
}

/// The transport handshake message exchanged on every TCP connection.
///
/// Since Phase 3 the Hello carries the peer's long-term Ed25519 public key, a
/// fresh ephemeral X25519 public key, and a signature binding them to this
/// device. The signature proves possession of the identity key; the trust
/// store binds the identity key to the device ID.
///
/// All new fields are optional/hex so a Hello from an older UniKey instance
/// on the same LAN still parses (it simply reports no keys, and the connection
/// is treated as legacy/unauthenticated).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    pub protocol_version: u32,
    pub device_id: Uuid,
    pub device_name: String,
    pub platform: String,

    /// Long-term Ed25519 public key (32 B, hex). Empty = not advertised.
    #[serde(serialize_with = "ser_key32", deserialize_with = "de_key32")]
    pub ed25519_public_key: [u8; 32],
    /// Ephemeral X25519 public key for this connection (32 B, hex).
    #[serde(serialize_with = "ser_key32", deserialize_with = "de_key32")]
    pub x25519_public_key: [u8; 32],
    /// Wall-clock epoch millis at signature time (freshens the handshake).
    pub timestamp: u64,
    /// Ed25519 signature over `device_id || x25519_public_key || timestamp` (64 B, hex).
    #[serde(serialize_with = "ser_key64", deserialize_with = "de_key64")]
    pub signature: [u8; 64],
}

impl Default for Hello {
    fn default() -> Self {
        Self {
            protocol_version: 1,
            device_id: Uuid::nil(),
            device_name: String::new(),
            platform: String::new(),
            ed25519_public_key: [0; 32],
            x25519_public_key: [0; 32],
            timestamp: 0,
            signature: [0; 64],
        }
    }
}

impl Hello {
    /// True when this Hello carries a complete key-exchange material
    /// (public key + signature), i.e. it came from a Phase 3 daemon.
    pub fn is_secure(&self) -> bool {
        !self.ed25519_public_key.iter().all(|b| *b == 0)
            && !self.x25519_public_key.iter().all(|b| *b == 0)
            && !self.signature.iter().all(|b| *b == 0)
    }
}
