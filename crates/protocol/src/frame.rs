//! Encrypted frame codec — the on-the-wire unit after the Phase 3 handshake.
//!
//! ```text
//!  0     1        2          10         22
//!  +-----+--------+----------+----------+------------------------------+
//!  | ver | flags  | counter  | nonce    | ciphertext (ChaCha20-Poly1305)|
//!  | (1) | (1)    | (8, BE)  | (12)     |   = plaintext || 16-byte tag  |
//!  +-----+--------+----------+----------+------------------------------+
//!  \──────────── AAD (10 bytes) ─────────/
//! ```
//!
//! The inner plaintext is a normal [`Packet`] (version | type | len | payload),
//! so the payload framing rules are unchanged — only the outer wrapping is
//! encrypted. The 10-byte AAD binds the header (version/flags/counter) to the
//! ciphertext, preventing header re-writing.

use anyhow::{bail, Result};

/// Header size: version + flags + counter + nonce.
pub const FRAME_HEADER_LEN: usize = 2 + 8 + 12;
/// Frame protocol version.
pub const FRAME_VERSION: u8 = 1;
/// Flag bit indicating the frame is authenticated-encrypted.
pub const FLAG_ENCRYPTED: u8 = 0x01;

/// A single sealed transport frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedFrame {
    /// Monotonic per-direction counter (part of the nonce + replay check).
    pub counter: u64,
    /// 12-byte AEAD nonce (8-byte counter + 4 random bytes).
    pub nonce: [u8; 12],
    /// Sealed plaintext: `ciphertext || 16-byte tag`.
    pub ciphertext: Vec<u8>,
}

impl EncryptedFrame {
    /// Serialize the frame (without the TCP length prefix — the transport adds it).
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(FRAME_HEADER_LEN + self.ciphertext.len());
        out.push(FRAME_VERSION);
        out.push(FLAG_ENCRYPTED);
        out.extend_from_slice(&self.counter.to_be_bytes());
        out.extend_from_slice(&self.nonce);
        out.extend_from_slice(&self.ciphertext);
        out
    }

    /// The AAD bytes authenticated by the AEAD: version + flags + counter.
    pub fn aad(&self) -> [u8; 10] {
        let mut aad = [0u8; 10];
        aad[0] = FRAME_VERSION;
        aad[1] = FLAG_ENCRYPTED;
        aad[2..10].copy_from_slice(&self.counter.to_be_bytes());
        aad
    }

    /// Parse a frame from a buffer that contains exactly one frame
    /// (the transport is responsible for splitting on the length prefix).
    pub fn decode(buf: &[u8]) -> Result<Self> {
        if buf.len() < FRAME_HEADER_LEN {
            bail!("frame too short: {} bytes", buf.len());
        }
        if buf[0] != FRAME_VERSION {
            bail!("unsupported frame version {}", buf[0]);
        }
        let counter = u64::from_be_bytes(buf[2..10].try_into().unwrap());
        let nonce: [u8; 12] = buf[10..22].try_into().unwrap();
        let ciphertext = buf[22..].to_vec();
        Ok(Self {
            counter,
            nonce,
            ciphertext,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_encode_decode() {
        let frame = EncryptedFrame {
            counter: 42,
            nonce: [7u8; 12],
            ciphertext: vec![1, 2, 3, 4, 5],
        };
        let bytes = frame.encode();
        let decoded = EncryptedFrame::decode(&bytes).expect("decode");
        assert_eq!(decoded, frame);
        assert_eq!(bytes.len(), FRAME_HEADER_LEN + 5);
    }

    #[test]
    fn aad_is_stable_and_binds_counter() {
        let a = EncryptedFrame {
            counter: 1,
            nonce: [0u8; 12],
            ciphertext: vec![],
        };
        let b = EncryptedFrame {
            counter: 2,
            nonce: [0u8; 12],
            ciphertext: vec![],
        };
        assert_eq!(a.aad()[0], FRAME_VERSION);
        assert_ne!(a.aad(), b.aad(), "counter must be authenticated");
    }

    #[test]
    fn rejects_short_and_bad_version() {
        assert!(EncryptedFrame::decode(&[1]).is_err());
        assert!(EncryptedFrame::decode(&[0; FRAME_HEADER_LEN]).is_err(), "version 0 rejected");
    }
}
