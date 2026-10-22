//! # UniKey cryptography core (Phase 3)
//!
//! Everything security-sensitive in UniKey lives behind this crate so the
//! rest of the codebase never touches raw key material or primitives:
//!
//! * **Identity** — Ed25519 long-term keypair that identifies a device
//!   forever. Used to sign the transport handshake (proves the peer owns the
//!   public key it claims) and to authenticate pairing.
//! * **Key agreement** — ephemeral X25519 per connection, giving forward
//!   secrecy: session keys are derived fresh on every connection and a
//!   compromise of one session never decrypts another.
//! * **Session keys** — HKDF-SHA256 derives two independent 32-byte ChaCha20
//!   keys (one per direction) so a frame can never be replayed *back* at the
//!   sender (reflection attack).
//! * **Authenticated encryption** — ChaCha20-Poly1305 seals every transport
//!   frame. AAD binds the frame header + nonce counter to the ciphertext.
//! * **Replay protection** — a monotonic per-direction counter, enforced by a
//!   [`ReplayGuard`]. TCP preserves order, so strict monotonicity is exact;
//!   keys rotate on every connection so old frames never decrypt.
//!
//! ## Threat model
//!
//! An attacker on the LAN can observe and inject UDP/TCP traffic. They cannot
//! forge a trusted peer's Ed25519 signature, and they cannot decrypt frames
//! without the ephemeral X25519 key. PIN pairing bootstraps the first trust:
//! the PIN authenticates the initiator to the responder, and both sides store
//! each other's Ed25519 public key for all future handshakes.

pub mod aead;
pub mod hash;
pub mod kdf;
pub mod keypair;
pub mod replay;

pub use aead::{open, seal, Nonce, KEY_LEN as AEAD_KEY_LEN, NONCE_LEN, TAG_LEN};
pub use kdf::{derive_session_keys, derive_session_master, SessionKeys};
pub use keypair::{
    generate_identity_keypair, generate_keypair, save_keypair, verify_signature, verify_handshake, sign_handshake, IdentityKey, X25519KeyPair, KEY_BYTES,
    SIGNATURE_BYTES,
};
pub use replay::ReplayGuard;

/// Length of every public key / hash in bytes.
pub const KEY_BYTES_32: usize = 32;
