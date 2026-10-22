//! HKDF-SHA256 session-key derivation.
//!
//! From the raw X25519 shared secret we derive a 64-byte master, then split it
//! into two independent 32-byte ChaCha20 keys — one per direction. Separating
//! directions means a frame captured on the wire can never be replayed back at
//! its sender (a classic reflection attack).

use hkdf::Hkdf;
use sha2::Sha256;
use uuid::Uuid;

pub const KEY_LEN: usize = 32;
/// 64 bytes: key_low_to_high (32) || key_high_to_low (32).
pub const SESSION_KEY_BYTES: usize = 64;

/// Per-direction ChaCha20-Poly1305 keys for one connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionKeys {
    /// Key used to encrypt frames *sent to* the peer.
    pub send_key: [u8; KEY_LEN],
    /// Key used to decrypt frames *received from* the peer.
    pub recv_key: [u8; KEY_LEN],
}

/// Derive the 64-byte session master from the X25519 shared secret.
///
/// The HKDF salt is the canonical (sorted) pair of device IDs, so both peers
/// derive identical material regardless of who initiated the connection. The
/// info string namespaces UniKey's derivation so it can never collide with
/// other uses of the same shared secret.
pub fn derive_session_master(
    shared_secret: &[u8; KEY_LEN],
    device_a: &Uuid,
    device_b: &Uuid,
) -> [u8; SESSION_KEY_BYTES] {
    let (low, high) = if device_a < device_b {
        (device_a, device_b)
    } else {
        (device_b, device_a)
    };

    let mut salt = [0u8; 32];
    for (i, b) in low.as_bytes().iter().chain(high.as_bytes()).enumerate() {
        salt[i] = *b;
    }

    let hk = Hkdf::<Sha256>::new(Some(&salt), shared_secret);
    let mut okm = [0u8; SESSION_KEY_BYTES];
    hk.expand(b"unikey/session/v1", &mut okm)
        .expect("HKDF expansion of a fixed size is infallible");
    okm
}

/// Select the per-direction traffic keys for a connection from the local
/// device's point of view.
///
/// `master[0..32]` encrypts traffic from the *lower* device ID to the higher
/// one; `master[32..64]` the reverse. Both peers call this and arrive at the
/// same `send_key`/`recv_key` pair.
pub fn derive_session_keys(
    master: &[u8; SESSION_KEY_BYTES],
    local_id: &Uuid,
    peer_id: &Uuid,
) -> SessionKeys {
    let low_to_high: [u8; KEY_LEN] = master[..32].try_into().unwrap();
    let high_to_low: [u8; KEY_LEN] = master[32..].try_into().unwrap();

    if local_id < peer_id {
        SessionKeys {
            send_key: low_to_high,
            recv_key: high_to_low,
        }
    } else {
        SessionKeys {
            send_key: high_to_low,
            recv_key: low_to_high,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shared() -> [u8; 32] {
        [42u8; 32]
    }

    #[test]
    fn both_sides_derive_identical_keys() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let master = derive_session_master(&shared(), &a, &b);

        let from_a = derive_session_keys(&master, &a, &b);
        let from_b = derive_session_keys(&master, &b, &a);

        // A's send key is B's recv key and vice versa.
        assert_eq!(from_a.send_key, from_b.recv_key);
        assert_eq!(from_a.recv_key, from_b.send_key);
    }

    #[test]
    fn direction_keys_are_independent() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let master = derive_session_master(&shared(), &a, &b);
        let keys = derive_session_keys(&master, &a, &b);
        assert_ne!(keys.send_key, keys.recv_key);
    }

    #[test]
    fn different_shared_secrets_yield_different_keys() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let m1 = derive_session_master(&[1u8; 32], &a, &b);
        let m2 = derive_session_master(&[2u8; 32], &a, &b);
        let k1 = derive_session_keys(&m1, &a, &b);
        let k2 = derive_session_keys(&m2, &a, &b);
        assert_ne!(k1.send_key, k2.send_key);
    }

    #[test]
    fn master_is_stable_across_call_orders() {
        let a = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
        let b = Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap();
        let m1 = derive_session_master(&shared(), &a, &b);
        let m2 = derive_session_master(&shared(), &b, &a);
        assert_eq!(m1, m2);
    }
}
