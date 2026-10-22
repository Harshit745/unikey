//! ChaCha20-Poly1305 authenticated encryption for transport frames.
//!
//! Wire format of a sealed frame (see `crates/transport` for the length prefix
//! that frames these on the TCP stream):
//!
//! ```text
//!  0               8               20              24
//!  +---------------+----------------+---------------+----------------------+
//!  | ver | flags   | counter (BE)   | 4B random     | ciphertext || tag    |
//!  +---------------+----------------+---------------+----------------------+
//!  \────────────── nonce (12 B) ────────────────/
//! ```
//!
//! The AEAD AAD authenticates the header bytes (version/flags) and the counter
//! portion of the nonce, so an attacker cannot re-order or re-header a frame
//! without breaking authentication.

use anyhow::{bail, Result};
use chacha20poly1305::{
    aead::{Aead, Payload},
    ChaCha20Poly1305, KeyInit,
};
use rand_core::{OsRng, RngCore};

pub const KEY_LEN: usize = 32;
pub const NONCE_LEN: usize = 12;
pub const TAG_LEN: usize = 16;
/// Upper bound on a single decrypted payload (4 MiB, mirrors the transport).
pub const MAX_PLAINTEXT: usize = 4 * 1024 * 1024;

/// A 12-byte ChaCha20-Poly1305 nonce: 8-byte big-endian counter + 4 random
/// bytes. The counter makes nonces unique per direction/key and is what the
/// [`crate::ReplayGuard`] checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nonce(pub [u8; NONCE_LEN]);

impl Nonce {
    /// Build a nonce from a monotonic counter + fresh random tail.
    pub fn from_counter(counter: u64, random: [u8; 4]) -> Self {
        let mut b = [0u8; NONCE_LEN];
        b[..8].copy_from_slice(&counter.to_be_bytes());
        b[8..].copy_from_slice(&random);
        Self(b)
    }

    /// A fully random nonce (for one-shot authenticated messages).
    pub fn random() -> Self {
        let mut random = [0u8; 4];
        OsRng.fill_bytes(&mut random);
        Self::from_counter(0, random)
    }

    /// The embedded monotonic counter.
    pub fn counter(&self) -> u64 {
        u64::from_be_bytes(self.0[..8].try_into().unwrap())
    }

    pub fn as_bytes(&self) -> &[u8; NONCE_LEN] {
        &self.0
    }
}

/// Seal `plaintext`, returning `ciphertext || 16-byte tag`.
///
/// ChaCha20-Poly1305 encryption cannot fail for valid inputs; it is infallible
/// here by construction (the `expect` is documented as unreachable).
pub fn seal(
    key: &[u8; KEY_LEN],
    nonce: &Nonce,
    aad: &[u8],
    plaintext: &[u8],
) -> Vec<u8> {
    let cipher = ChaCha20Poly1305::new(key.into());
    cipher
        .encrypt(
            chacha20poly1305::Nonce::from_slice(nonce.as_bytes()),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .expect("ChaCha20-Poly1305 encryption is infallible for valid inputs")
}

/// Open a sealed frame. Returns the plaintext, or an error when the AEAD tag
/// does not verify (tampered ciphertext, wrong key, or nonce/AAD mismatch).
pub fn open(
    key: &[u8; KEY_LEN],
    nonce: &Nonce,
    aad: &[u8],
    ciphertext_with_tag: &[u8],
) -> Result<Vec<u8>> {
    if ciphertext_with_tag.len() < TAG_LEN {
        bail!("ciphertext shorter than authentication tag ({TAG_LEN} bytes)");
    }
    let cipher = ChaCha20Poly1305::new(key.into());
    let pt = cipher
        .decrypt(
            chacha20poly1305::Nonce::from_slice(nonce.as_bytes()),
            Payload {
                msg: ciphertext_with_tag,
                aad,
            },
        )
        .map_err(|_| anyhow::anyhow!("AEAD authentication failed"))?;
    if pt.len() > MAX_PLAINTEXT {
        bail!("decrypted payload exceeds maximum size");
    }
    Ok(pt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> [u8; 32] {
        [7u8; 32]
    }

    #[test]
    fn round_trip() {
        let k = key();
        let nonce = Nonce::from_counter(1, [0xde, 0xad, 0xbe, 0xef]);
        let ct = seal(&k, &nonce, b"aad", b"hello world");
        let pt = open(&k, &nonce, b"aad", &ct).expect("open");
        assert_eq!(pt, b"hello world");
    }

    #[test]
    fn tampered_ciphertext_is_rejected() {
        let k = key();
        let nonce = Nonce::from_counter(1, [0u8; 4]);
        let mut ct = seal(&k, &nonce, b"", b"secret payload");
        let mid = ct.len() / 2;
        ct[mid] ^= 0x01;
        assert!(open(&k, &nonce, b"", &ct).is_err());
    }

    #[test]
    fn wrong_key_and_nonce_fail() {
        let k = key();
        let nonce = Nonce::from_counter(1, [0u8; 4]);
        let ct = seal(&k, &nonce, b"", b"data");

        let wrong = [8u8; 32];
        assert!(open(&wrong, &nonce, b"", &ct).is_err());

        let nonce2 = Nonce::from_counter(2, [0u8; 4]);
        assert!(open(&k, &nonce2, b"", &ct).is_err());
    }

    #[test]
    fn aad_is_authenticated() {
        let k = key();
        let nonce = Nonce::from_counter(1, [0u8; 4]);
        let ct = seal(&k, &nonce, b"header", b"payload");
        assert!(open(&k, &nonce, b"tampered", &ct).is_err());
        // Reopening with the right AAD works.
        assert_eq!(open(&k, &nonce, b"header", &ct).unwrap(), b"payload");
    }

    #[test]
    fn truncated_frame_is_rejected() {
        let k = key();
        let nonce = Nonce::from_counter(1, [0u8; 4]);
        let ct = seal(&k, &nonce, b"", b"data");
        assert!(open(&k, &nonce, b"", &ct[..5]).is_err());
        assert!(open(&k, &nonce, b"", &[]).is_err());
    }

    #[test]
    fn counter_parses_correctly() {
        let n = Nonce::from_counter(0x0102_0304_0506_0708, [9, 10, 11, 12]);
        assert_eq!(n.counter(), 0x0102_0304_0506_0708);
        assert_eq!(n.as_bytes()[8..], [9, 10, 11, 12]);
    }

    #[test]
    fn oversized_plaintext_is_rejected() {
        let k = key();
        let nonce = Nonce::from_counter(1, [0u8; 4]);
        let big = vec![0u8; MAX_PLAINTEXT + 1];
        let ct = seal(&k, &nonce, b"", &big);
        assert!(open(&k, &nonce, b"", &ct).is_err());
    }
}
