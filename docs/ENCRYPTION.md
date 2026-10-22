# UniKey Encryption

UniKey is end-to-end encrypted between devices. This document describes the key material, the handshake, the cipher suite, and the operational protections (replay, rotation, flooding, path security). Code lives in `crates/crypto`, `crates/protocol`, and `crates/transport`.

## Threat model

- **In scope** — an attacker on the same LAN who can observe, modify, inject, or replay packets between two devices.
- **Out of scope** — an attacker with code execution on either device, or physical access to an unlocked device. A device that has already been trusted can always read what it is sent.

## Key material

| Key | Type | Lifetime | Purpose |
|-----|------|----------|---------|
| Identity key | Ed25519 signing keypair | Persistent, per device (file in the keys dir) | Device identity; signs handshakes and pairing exchanges; verifies trust |
| X25519 keypair | Ephemeral per handshake | One handshake | Key agreement |
| Session keys | 2 × ChaCha20-Poly1305 keys | One session (rotatable) | Bulk encryption, one key per direction |

Identity keys are stored on disk; X25519 keys are ephemeral and never persisted. Nothing else is needed for forward secrecy because the per-session keys are derived fresh from an ephemeral X25519 exchange on every connection.

## Trust establishment (pairing)

Implemented in `crates/pairing`. In short:

1. Two devices meet via discovery.
2. They perform a PIN or QR pairing flow that authenticates each other's **identity public keys**.
3. On success each device stores the other's identity key in its **persistent trust store**.

Every later handshake verifies the peer's signature against this stored identity key, so the trust survives restarts and no third party can impersonate a trusted device without its private key.

## Handshake and key agreement

Per connection:

1. Both sides generate an ephemeral **X25519** keypair.
2. Each side sends a `Hello` containing its identity (Ed25519) public key, its ephemeral X25519 public key, and a signature over the whole message.
3. Each side verifies the peer's signature against the **known** identity key and then computes `X25519(own_ephemeral, peer_ephemeral)` → shared secret.
4. The shared secret is fed through **HKDF-SHA256** to produce a 64-byte session master, then split into two independent 32-byte traffic keys:

```
HKDF(ikm = X25519(ephemeral, peer_ephemeral),
     salt = sorted device-ID pair,
     info = "unikey/session/v1")  →  64 bytes

master[0..32]  →  key for traffic from the lower device ID to the higher
master[32..64] →  key for traffic from the higher device ID to the lower
```

Separating directions means a frame captured on the wire can never be reflected back at its sender. Both peers compute the same keys regardless of who dialed, because the salt is the **canonically sorted** pair of device IDs.

## Cipher suite

| Component | Choice |
|-----------|--------|
| Key agreement | X25519 (Curve25519 ECDH) |
| Identity / signatures | Ed25519 |
| KDF | HKDF-SHA256 |
| AEAD | ChaCha20-Poly1305 |
| Integrity / dedup hash | SHA-256 (file transfers) |
| RNG | OS CSPRNG |

### AEAD frame

Every application packet after the handshake is sealed:

```
 ver | flags | counter(8) | nonce(12) | ChaCha20-Poly1305(plaintext || tag16)
 \──────── AAD (ver|flags|counter) ────────/
```

- **Nonce** = 8-byte monotonic per-direction counter + 4 random bytes. The counter both keys the replay guard and guarantees nonce uniqueness under a given key.
- **AAD** binds the header to the ciphertext, so an attacker cannot rewrite the counter or version.

## Operational protections

- **Replay protection** — a per-direction monotonic counter is checked against a sliding `ReplayGuard` before decryption; frames with a counter already seen or older than the window are dropped. This blocks replay of old valid frames and out-of-order injection.
- **Key rotation** — sessions can re-key on a configurable interval (`encryption.key_rotation_minutes`), deriving a fresh session master and keys without re-pairing.
- **Malformed packet handling** — every frame is validated at decode time (length checks, version checks, AEAD authentication). A frame that fails authentication is rejected with a logged error; it never reaches a service.
- **Flooding / rate limiting** — pairing is rate-limited to prevent brute-force PIN attempts; the API is localhost-only; the transport closes handshakes that time out or send garbage.
- **Pairing required** — by default, an unknown device cannot open a session without completing pairing (`encryption.require_pairing`).

## Validation, not just encryption

Encryption guarantees confidentiality and integrity, but UniKey additionally validates every payload at the application layer:

- File names are **sanitized** (`..`, separators, absolute paths rejected) before being written to disk — no directory traversal.
- Transfer chunk counts are cross-checked against the declared file size, and the reassembled file is **SHA-256 verified** against the metadata before it is considered complete.
- Every packet is size- and type-checked before dispatch.

## What is NOT encrypted

Discovery beacons are intentionally plaintext — they carry only identity (name, platform, device id, port) and must be readable so unknown devices can find each other to start pairing. Nothing else is sent in plaintext.

## See Also

- [NETWORKING.md](NETWORKING.md) — wire format and handshake
- [ARCHITECTURE.md](ARCHITECTURE.md) — where the crypto sits
