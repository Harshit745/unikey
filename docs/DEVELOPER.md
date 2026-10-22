# UniKey Developer Guide

Everything you need to build, run, and extend UniKey from source.

## Prerequisites

- **Rust** 1.85+ (edition 2024 workspace)
- **Node.js** 22+ and npm (for the desktop shell)
- **Tauri v2** system dependencies on macOS: Xcode Command Line Tools (`xcode-select --install`). On Windows: the MSVC build tools (see [BUILD.md](BUILD.md)).

## Repository layout

```
crates/          Rust library crates (one concern each — see ARCHITECTURE.md)
runtime/daemon   The daemon binary (API, wiring, config, logging)
apps/desktop     Tauri v2 desktop shell (React frontend + Rust host)
scripts/         Cross-platform build/test/release tooling
config/          Shipping config templates
assets/          Icons and static assets copied into the release folder
docs/            This documentation
```

## Daily workflow

```bash
# Fast checks (well under a minute)
cargo test --workspace
cargo clippy --workspace --all-targets   # must stay at 0 warnings

# Run the daemon in development (standard per-user config, console logs)
cargo run -p daemon

# Then hit the API
curl -s 127.0.0.1:45895/health

# Run the desktop shell (frontend + daemon together)
cd apps/desktop
npm install
npm run tauri dev
```

The desktop shell launches its own daemon. To develop against a manually-run
daemon instead, keep it running and the shell will detect it (the health check
drives the UI's "daemon online" state).

## Where things live

| Concern | Location |
|---------|----------|
| Wire protocol (message types, hello, frames) | `crates/protocol` |
| Cryptography (keys, AEAD, KDF, replay) | `crates/crypto` |
| TCP transport (connection, session, handshake) | `crates/transport` |
| Discovery (UDP beacons, peer registry) | `crates/discovery` |
| Pairing + trust store | `crates/pairing` |
| Services: clipboard, input, files | `crates/clipboard`, `crates/input`, `crates/files` |
| Event bus + typed events | `crates/events` |
| HTTP API, config, logging, startup | `runtime/daemon` |
| GUI (React pages, API client) | `apps/desktop/src` |
| GUI host (tray, notifications, daemon spawn) | `apps/desktop/src-tauri` |

## Adding an HTTP endpoint

1. Add the handler in `runtime/daemon/src/api.rs` (return `ApiResponse<T>`).
2. Register the route in `router()` with the `:id`-style path syntax.
3. Add the typed request/response struct (with `serde` derives).
4. Add a test next to the existing handler tests (there is a `test_state()`
   helper that wires up an in-memory state with temp dirs).
5. Expose it from the frontend in `apps/desktop/src/api/client.ts` +
   `apps/desktop/src/api/types.ts`.

The OpenAPI document (`GET /api/v1/openapi.json`) is generated from the route
table and stays in sync automatically; hand-written entries are not needed.

## Adding a service

1. Create (or reuse) a crate under `crates/` that implements
   `services::Service` — the trait gives you a lifecycle (`start`/`stop`), a
   context with the session manager and shutdown token, and a dispatch hook
   for inbound message types.
2. Register it in `runtime/daemon/src/main.rs` and (if it has a toggle) in
   `ServicesConfig`.
3. Publish `AppEvent`s for anything the GUI or notifications care about.

## Config & persistence

- Default config is written to the config dir on first run (`unikey.toml`).
- **Never delete or overwrite a user's malformed config** — log a warning and
  fall back to defaults (see `runtime/daemon/src/config.rs`).
- New config sections must use `#[serde(default)]` so older config files keep
  loading; bump `CONFIG_SCHEMA_VERSION` only when fields are removed/renamed.

## Testing

See [TESTING.md](TESTING.md). Quick summary:

- Unit tests live next to the code (`#[cfg(test)]`), including the async
  handshake/session/transfer tests.
- Integration tests for real end-to-end flows are in `crates/transport/tests/`
  and `crates/files/tests/`.
- The API has handler tests that boot the router in-process with a test state.

## Conventions

- **No direct socket ownership outside the daemon.** GUI/CLI consumers use the
  HTTP API only.
- **No `unwrap()`/`panic!` on input paths** — network and file input always
  goes through validation (see `SECURITY.md`).
- Keep `cargo clippy --all-targets` at **0 warnings** and the test suite green
  before finishing any change.
- Match the surrounding code's comment density and style — heavy doc comments
  in public APIs, pragmatic inline comments elsewhere.

## Full release build

```bash
./scripts/build.sh --clean        # macOS / Linux
.\scripts\build.ps1 -Clean        # Windows (PowerShell)
```

Produces `release/UniKey/` (portable folder) + a `.zip`. See [RELEASE.md](RELEASE.md).
