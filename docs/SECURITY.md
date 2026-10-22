# UniKey Security

UniKey is built around a few hard rules. This document maps each rule to the
code that enforces it. The cryptographic details live in
[ENCRYPTION.md](ENCRYPTION.md); here we focus on *policies* and *attack
surfaces*.

## Security rules

1. **Never trust unknown devices.** Discovery is open (it must be, so devices
   can find each other), but an unknown device cannot open a session, read
   clipboard content, inject input, or receive files. Every TCP session is
   authenticated against the persistent trust store before any application
   data flows.
2. **Protect against replay.** Every encrypted frame carries a monotonic
   per-direction counter checked by a `ReplayGuard` before decryption
   (`crates/crypto/src/replay.rs`). Replayed, reordered, or stale frames are
   dropped. The counter is also part of the AEAD nonce, so the same
   (key, counter) can never be reused.
3. **Protect against malformed packets.** Every inbound byte is validated
   before dispatch:
   - Frame decode checks length, version, and AEAD tag
     (`crates/protocol/src/frame.rs`).
   - Message type is checked against the known enum; unknown codes are
     rejected (`crates/protocol/src/message.rs`).
   - Payload size limits are enforced after decryption
     (`crates/crypto/src/aead.rs`).
   - A handshake that does not complete, or sends garbage, is closed on a
     timeout and never admitted.
4. **Protect against flooding.**
   - **Pairing** sessions are short-lived, single-code, and rate-limited —
     a 6-digit code cannot be brute-forced over the LAN within its lifetime.
   - **API** is bound to `127.0.0.1` only; no LAN host can reach it.
   - **Inbound queues** are bounded; overflow is dropped and logged, never
     unbounded-buffered.
5. **No arbitrary command execution.** Input forwarding injects **key and
   pointer events** only, through platform input APIs — it never executes
   commands, never spawns processes, and never accepts code.
6. **No directory traversal.** Received file names pass through a sanitizer
   that rejects `..`, path separators, and absolute paths before any write
   (`crates/files`). Downloads land in the configured download dir, never a
   caller-supplied path.
7. **Validate every packet. Validate every path.** Beyond framing checks,
   application payloads are cross-validated: file sizes vs. chunk counts,
   reassembled files vs. the SHA-256 declared in metadata, transfer IDs vs.
   known transfers, peers vs. the trusted set.
8. **Nothing beyond discovery travels in plaintext.** Beacons are plaintext
   identity (name, platform, device id, port) by necessity. Everything after
   the handshake is AEAD-encrypted. Clipboard, input, file contents, pairing
   messages — all encrypted.

## Attack surfaces

| Surface | Exposure | Mitigation |
|---------|----------|------------|
| UDP discovery (45893) | LAN broadcast | Identity only; no keys, no session data. Malformed/foreign packets are ignored. |
| TCP transport (45892) | LAN inbound | Handshake-first; signature verified against trust store; timeout closes idle/handshake-stuck connections; encrypted framing after hello. |
| HTTP API (45895) | Localhost only | Bound to 127.0.0.1; never exposed. Trusted local processes only. |
| Files on disk | Local user | Identity keys + trust store in the user config dir with restrictive intent; received files sanitized and checksum-verified. |

## Operational hardening

- **Identity keys** are generated from the OS CSPRNG and persisted per device.
  Losing them loses trust; deleting the trust store revokes every peer.
- **Key rotation** is available per session (`encryption.key_rotation_minutes`)
  so a long-lived session does not accumulate ciphertext under one key.
- **Logging** is local; log rotation bounds disk usage. Logs do not contain
  session keys or clipboard contents (only previews/events).

## Reporting

UniKey is pre-release. If you find a weakness, file an issue with the
affected surface (discovery / transport / API / files) and a minimal repro.
Do not share exploit details publicly before maintainers have had a chance to
respond.

## See Also

- [ENCRYPTION.md](ENCRYPTION.md) — the cryptography behind these guarantees
- [NETWORKING.md](NETWORKING.md) — wire protocol and boundaries
