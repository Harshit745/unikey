//! SHA-256 helpers used for integrity verification (file transfer chunks)
//! and content fingerprinting (clipboard loop prevention).

use sha2::{Digest, Sha256};

pub const HASH_BYTES: usize = 32;

/// SHA-256 digest of `data`.
pub fn sha256(data: &[u8]) -> [u8; HASH_BYTES] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let out = hasher.finalize();
    let mut arr = [0u8; HASH_BYTES];
    arr.copy_from_slice(&out);
    arr
}

/// Lowercase hex encoding of the SHA-256 digest (for display/logs).
pub fn sha256_hex(data: &[u8]) -> String {
    let d = sha256(data);
    d.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vector() {
        // SHA-256("abc") — NIST test vector.
        let digest = sha256(b"abc");
        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(sha256_hex(b"abc"), expected);
        assert_eq!(digest.len(), 32);
    }

    #[test]
    fn deterministic_and_distinct() {
        assert_eq!(sha256(b"same"), sha256(b"same"));
        assert_ne!(sha256(b"one"), sha256(b"two"));
        assert_ne!(sha256(b"a"), sha256(b"aa"));
    }
}
