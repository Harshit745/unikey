# UniKey Testing

How to run and extend the test suite. Everything is Rust (the frontend is type-checked at build time with `tsc -b`).

## Running

```bash
# All crates, all targets (well under a minute)
cargo test --workspace

# A single crate
cargo test -p transport

# One test by name
cargo test -p transport handshake_rejects_tampered_signature

# Lints — must stay at 0 warnings
cargo clippy --workspace --all-targets

# Frontend type-check + production build
cd apps/desktop && npm run build

# Desktop host crate (tray, notifications, daemon spawn)
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml
```

There is a wrapper for the workspace suite: `./scripts/test.sh` (macOS/Linux)
and `.\scripts\test.ps1` (Windows). CI runs the same commands.

## What is covered

| Layer | Where | What it proves |
|-------|-------|----------------|
| Crypto | `crates/crypto` | KDF determinism (both sides derive identical keys), AEAD seal/open + tamper rejection, Ed25519 sign/verify, replay guard rejects replayed counters, hash correctness, keypair round-trips |
| Protocol | `crates/protocol` | Message type round-trips, hello packet shape + signature binding, frame encode/decode, unknown-type rejection |
| Discovery | `crates/discovery` | Beacon/reply parsing, peer registry add/update/prune, platform parsing |
| Transport | `crates/transport` unit + `crates/transport/tests/secure_session.rs` | **End-to-end encrypted session over real TCP sockets** — two peers establish a session, tampered handshake signatures are rejected, encrypted packets round-trip, graceful disconnect tears the session down, lifecycle events publish |
| Pairing | `crates/pairing` | Code generation/validation + expiry, session state machine, cleanup, trust-store add/remove/rename/persist, bounded history, corrupted-store error, full service flow (pair → trust → survive reload), pending-response routing |
| Clipboard | `crates/clipboard` | Service with a mock backend: bounded history, preview truncation, inbound write + echo-suppression, malformed payload, non-clipboard messages ignored |
| Input | `crates/input` | Event mapping / forwarding plumbing |
| Files | `crates/files` unit + `crates/files/tests/transfer_e2e.rs` | **Full file transfer over the wire** — metadata → chunks → reassembly → SHA-256 verification; oversized metadata rejected |
| Events | `crates/events` | Envelope serialization with `type` discriminator, round-trips for every variant, bus history |
| Services | `crates/services` | Registry lifecycle (registered→running→stopped), packet routing to the right slot, unknown-slot no-op |
| Daemon API | `runtime/daemon` | Every HTTP endpoint returns the envelope; config PUT persists + live-swaps; pairing/tranfer edge cases; OpenAPI describes the API |
| Device | `crates/device` | Identity load/create, path resolution |
| Common | `crates/common` | Port invariants (distinct, in range) |

## How the integration tests work

The secure-session and transfer tests spin up two **in-process peer stacks**
(identity + connection manager + session manager, sometimes a real `TcpListener`
pair) on loopback, run the full handshake over a real TCP socket, then assert
on the decrypted traffic. They do **not** touch the network or the filesystem
outside temp dirs.

The daemon API tests boot the axum router in-process with a `test_state()`
helper (temp config/keys dirs, in-memory services) and drive it with
`tower::ServiceExt::oneshot` — no port is opened.

## Adding a test

1. Match the existing pattern: unit tests in `#[cfg(test)]` next to the code;
   cross-crate flows in `crates/<x>/tests/<name>.rs`.
2. Async tests use `#[tokio::test]`. The workspace has a single shared tokio
   so multi-threaded runtimes are fine.
3. Never depend on wall-clock timing for pass/fail (heartbeat/idle tests use
   short explicit durations).
4. Verify with `cargo test -p <crate>` and confirm `cargo clippy` stays clean.

## Acceptance gate

Before considering a change done:

- [ ] `cargo test --workspace` green
- [ ] `cargo clippy --workspace --all-targets` — 0 warnings
- [ ] `cd apps/desktop && npm run build` — clean `tsc` + Vite build
