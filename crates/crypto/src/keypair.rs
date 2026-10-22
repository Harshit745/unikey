//! Device identity keys (Ed25519) and per-connection ephemeral keys (X25519).

use anyhow::{bail, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::{OsRng, RngCore};
use uuid::Uuid;
use x25519_dalek::{PublicKey as XPublicKey, StaticSecret};

pub const KEY_BYTES: usize = 32;
pub const SIGNATURE_BYTES: usize = 64;

/// Long-term Ed25519 identity keypair. This is the device's permanent identity:
/// it is generated once, stored in the config directory, and every transport
/// handshake is signed with it so peers can verify *which* device is speaking.
#[derive(Clone)]
pub struct IdentityKey {
    signing: SigningKey,
}

impl std::fmt::Debug for IdentityKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never log secret key material.
        f.debug_struct("IdentityKey").field("public_key", &hex_fmt(&self.public_key())).finish()
    }
}

fn hex_fmt(bytes: &[u8; 32]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(64);
    for b in bytes.iter().take(8) {
        write!(out, "{b:02x}").unwrap();
    }
    out.push('…');
    out
}

impl IdentityKey {
    /// Generate a fresh identity keypair from the OS CSPRNG.
    pub fn generate() -> Self {
        Self {
            signing: SigningKey::generate(&mut OsRng),
        }
    }

    /// Build an identity from its raw 32-byte private key.
    pub fn from_bytes(bytes: [u8; KEY_BYTES]) -> Self {
        Self {
            signing: SigningKey::from_bytes(&bytes),
        }
    }

    /// The raw 32-byte private key (for persistence).
    pub fn to_bytes(&self) -> [u8; KEY_BYTES] {
        self.signing.to_bytes()
    }

    /// The 32-byte Ed25519 public key as it appears on the wire.
    pub fn public_key(&self) -> [u8; KEY_BYTES] {
        self.signing.verifying_key().to_bytes()
    }

    /// Sign a message, returning the 64-byte Ed25519 signature.
    pub fn sign(&self, message: &[u8]) -> [u8; SIGNATURE_BYTES] {
        let sig: Signature = self.signing.sign(message);
        sig.to_bytes()
    }

    /// Load an identity from a raw 32-byte private-key file (UniKey's layout).
    pub fn load_private_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        if bytes.len() != KEY_BYTES {
            bail!(
                "invalid private key length {} (expected {KEY_BYTES})",
                bytes.len()
            );
        }
        let mut arr = [0u8; KEY_BYTES];
        arr.copy_from_slice(&bytes);
        Ok(Self::from_bytes(arr))
    }
}

/// Verify an Ed25519 signature in constant time. Returns `false` for any
/// malformed key/signature instead of erroring, so callers cannot distinguish
/// attacker-controlled failures.
pub fn verify_signature(
    public_key: &[u8; KEY_BYTES],
    message: &[u8],
    signature: &[u8; SIGNATURE_BYTES],
) -> bool {
    let Ok(verifying) = VerifyingKey::from_bytes(public_key) else {
        return false;
    };
    let sig = Signature::from_bytes(signature);
    verifying.verify(message, &sig).is_ok()
}

/// The canonical bytes signed by the long-term key during the transport
/// handshake. Binds the ephemeral X25519 key AND the device ID to the current
/// handshake, so a captured (ephemeral, signature) pair cannot be replayed
/// against a different device ID, and the signature cannot be reused later
/// than `timestamp` by any honest peer (it does not expire per se — replay is
/// prevented because the ephemeral key changes every connection).
pub fn handshake_message(
    device_id: &Uuid,
    ephemeral_public: &[u8; KEY_BYTES],
    timestamp: u64,
) -> Vec<u8> {
    let mut msg = Vec::with_capacity(16 + KEY_BYTES + 8);
    msg.extend_from_slice(device_id.as_bytes());
    msg.extend_from_slice(ephemeral_public);
    msg.extend_from_slice(&timestamp.to_le_bytes());
    msg
}

/// Sign a handshake message with the long-term identity key.
pub fn sign_handshake(
    identity: &IdentityKey,
    device_id: &Uuid,
    ephemeral_public: &[u8; KEY_BYTES],
    timestamp: u64,
) -> [u8; SIGNATURE_BYTES] {
    identity.sign(&handshake_message(device_id, ephemeral_public, timestamp))
}

/// Verify a peer's handshake signature against its claimed public key.
pub fn verify_handshake(
    claimed_public: &[u8; KEY_BYTES],
    device_id: &Uuid,
    ephemeral_public: &[u8; KEY_BYTES],
    timestamp: u64,
    signature: &[u8; SIGNATURE_BYTES],
) -> bool {
    verify_signature(
        claimed_public,
        &handshake_message(device_id, ephemeral_public, timestamp),
        signature,
    )
}

// ── Backward-compatible generation API ─────────────────────────────────

/// Generate a fresh Ed25519 identity keypair. Returns `(signing_key, public)`.
pub fn generate_identity_keypair() -> Result<(SigningKey, [u8; KEY_BYTES])> {
    let IdentityKey { signing } = IdentityKey::generate();
    let public = signing.verifying_key().to_bytes();
    Ok((signing, public))
}

/// Legacy alias expected by older crates (device-manager, etc.).
pub fn generate_keypair() -> Result<(SigningKey, [u8; KEY_BYTES])> {
    generate_identity_keypair()
}

/// Persist a keypair to the two paths used by the device manager.
pub fn save_keypair(
    private_key: &SigningKey,
    public_key: &[u8; KEY_BYTES],
    private_path: impl AsRef<std::path::Path>,
    public_path: impl AsRef<std::path::Path>,
) -> Result<()> {
    // Ensure parent directories exist
    if let Some(parent) = private_path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Some(parent) = public_path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(private_path, private_key.to_bytes())?;
    std::fs::write(public_path, public_key)?;
    Ok(())
}

// ── X25519 ephemeral key agreement ─────────────────────────────────────

/// A single-connection ephemeral X25519 keypair (forward secrecy).
///
/// The secret half is zeroized on drop so a dump of process memory does not
/// leak the ability to decrypt the session.
pub struct X25519KeyPair {
    secret: StaticSecret,
    public: XPublicKey,
}

impl std::fmt::Debug for X25519KeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("X25519KeyPair")
            .field("public", &hex_fmt(&self.public_bytes()))
            .finish()
    }
}

impl X25519KeyPair {
    /// Generate a fresh ephemeral keypair.
    pub fn generate() -> Self {
        let mut bytes = [0u8; KEY_BYTES];
        OsRng.fill_bytes(&mut bytes);
        let secret = StaticSecret::from(bytes);
        let public = XPublicKey::from(&secret);
        Self { secret, public }
    }

    /// The 32-byte X25519 public key to send to the peer.
    pub fn public_bytes(&self) -> [u8; KEY_BYTES] {
        self.public.to_bytes()
    }

    /// Compute the X25519 shared secret with a peer's ephemeral public key.
    /// Both sides obtain the same 32 bytes.
    pub fn shared_secret(&self, peer_public: &[u8; KEY_BYTES]) -> [u8; KEY_BYTES] {
        let peer = XPublicKey::from(*peer_public);
        self.secret.diffie_hellman(&peer).to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_generates_and_round_trips() {
        let a = IdentityKey::generate();
        let b = IdentityKey::from_bytes(a.to_bytes());
        assert_eq!(a.public_key(), b.public_key());
    }

    #[test]
    fn signs_and_verifies() {
        let key = IdentityKey::generate();
        let msg = b"unikey handshake v1";
        let sig = key.sign(msg);
        assert!(verify_signature(&key.public_key(), msg, &sig));
    }

    #[test]
    fn rejects_tampered_message_and_key() {
        let key = IdentityKey::generate();
        let msg = b"the quick brown fox";
        let sig = key.sign(msg);

        let mut tampered = msg.to_vec();
        tampered[3] ^= 0x01;
        assert!(!verify_signature(&key.public_key(), &tampered, &sig));

        let other = IdentityKey::generate();
        assert!(!verify_signature(&other.public_key(), msg, &sig));
    }

    #[test]
    fn rejects_malformed_signature() {
        let key = IdentityKey::generate();
        let mut sig = key.sign(b"x");
        sig[0] ^= 0xff;
        assert!(!verify_signature(&key.public_key(), b"x", &sig));
    }

    #[test]
    fn handshake_binds_device_and_ephemeral() {
        let id = IdentityKey::generate();
        let device = Uuid::new_v4();
        let eph = X25519KeyPair::generate();
        let ts = 1_700_000_000u64;

        let sig = sign_handshake(&id, &device, &eph.public_bytes(), ts);
        assert!(verify_handshake(&id.public_key(), &device, &eph.public_bytes(), ts, &sig));

        // Wrong device ID / timestamp / ephemeral all fail.
        let other_device = Uuid::new_v4();
        assert!(!verify_handshake(&id.public_key(), &other_device, &eph.public_bytes(), ts, &sig));
        assert!(!verify_handshake(&id.public_key(), &device, &eph.public_bytes(), ts + 1, &sig));

        let eph2 = X25519KeyPair::generate();
        assert!(!verify_handshake(&id.public_key(), &device, &eph2.public_bytes(), ts, &sig));
    }

    #[test]
    fn x25519_both_sides_agree() {
        let a = X25519KeyPair::generate();
        let b = X25519KeyPair::generate();
        let s_a = a.shared_secret(&b.public_bytes());
        let s_b = b.shared_secret(&a.public_bytes());
        assert_eq!(s_a, s_b);
    }

    #[test]
    fn x25519_different_peers_differ() {
        let a = X25519KeyPair::generate();
        let b = X25519KeyPair::generate();
        let c = X25519KeyPair::generate();
        assert_ne!(a.shared_secret(&b.public_bytes()), a.shared_secret(&c.public_bytes()));
    }
}
