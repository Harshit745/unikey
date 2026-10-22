# UniKey Roadmap

## Phase 1 — Core Connectivity & MVP (🚧 In verification)

| Feature | Status | Crate |
|---------|--------|-------|
| UDP device discovery + peer registry | ✅ Verified locally on macOS | `discovery` |
| Binary packet protocol + encrypted frames | ✅ Done | `protocol` |
| TCP transport, signed handshake, heartbeat | ✅ Verified locally; recovery testing remains | `transport` |
| Authenticated encryption (Ed25519, X25519, HKDF, ChaCha20-Poly1305, replay) | ✅ Verified locally | `crypto` |
| Persistent device identity (Ed25519) | ✅ Done | `device` |
| Service registry (extensible) | ✅ Done | `services` |
| PIN pairing + persistent key-pinned trust store | ✅ Verified locally; QR journey unverified | `pairing` |
| Clipboard sync (text / HTML / images, loop prevention, history) | 🧪 Unit-tested; physical-device journey unverified | `clipboard` |
| Keyboard & mouse forwarding | 🧪 Unit-tested; OS permissions/device journey unverified | `input` |
| Chunked file transfer (SHA-256, resume, cancel, progress) | ✅ Core transfer verified locally; resume/cancel journey unverified | `files` |
| Live event system (EventBus + SSE) | ✅ Done | `events` |
| HTTP API + OpenAPI | ✅ Done | `runtime/daemon` |
| Config with live-swap (`PUT /api/v1/config`) | ✅ Done | `runtime/daemon` |
| Logging + rotation | ✅ Tested; notifications and tray not interactively verified | `runtime/daemon`, `apps/desktop` |
| **Tauri v2 desktop shell** (React frontend) | 🧪 Web build and host tests pass; packaged UI journey unverified | `apps/desktop` |
| Single-folder portable builds + CI (Windows / macOS / Linux) | 🧪 Configured; current audit verified only local macOS builds | `scripts/`, `.github/` |

## Phase 2 — Productivity (📋 Stubbed)

| Feature | Crate | Description |
|---------|-------|-------------|
| Universal File Explorer | `phase2::StubFileExplorer` | Virtual drive mounting across connected machines |
| Instant File Transfer (resumable) | (part of `files`) | ✅ Resume support + SHA-256 verification landed in the MVP |
| Shared Clipboard Search | `phase2::StubSearchService` | Searchable encrypted clipboard history |
| Universal Search Overlay | `phase2::StubSearchService` | Ctrl+Space cross-device search (files, apps, clips) |
| Shared Assets | `phase2::StubAssetSync` | Shared downloads folder, camera roll auto-sync |

## Phase 3 — Universal Control (📋 Stubbed)

| Feature | Crate | Description |
|---------|-------|-------------|
| Cross-Device Audio | `phase3::StubAudioRouting` | Route audio between devices (desktop → speakers) |
| Mic & Webcam Sharing | `phase3::StubAudioRouting` | Share camera/mic across devices |
| Window Teleportation | `phase3::StubMediaShare` | Session state migration across OS bounds |
| Virtual Storage Pool | `phase3::StubVirtualStorage` | Unify multiple drives into a single volume |

## Phase 4 — Collaboration (📋 Stubbed)

| Feature | Crate | Description |
|---------|-------|-------------|
| Multi-User Cursors | `phase4::StubCollaboration` | Live cursor sharing across team devices |
| Shared Whiteboard | `phase4::StubCollaboration` | Real-time collaborative board |
| Guest Device Access | `phase4::StubWorkspace` | Temporary trusted device access |
| Team Clipboard | `phase4::StubWorkspace` | Shared clipboard across workspace members |

## Phase 5 — AI Operating Layer (📋 Stubbed)

| Feature | Crate | Description |
|---------|-------|-------------|
| Device Intelligence | `phase5::StubAiContext` | AI aware of CPU/RAM/battery/network for workload routing |
| AI Clipboard Search | `phase5::StubAiClipboard` | Natural language clipboard search |
| Auto-Summarize | `phase5::StubAiClipboard` | Auto-categorize and summarize copied text |
| Voice Commands | `phase5::StubAutomation` | "Move this to desktop" style commands |
| Trigger Rules | `phase5::StubAutomation` | Wi-Fi connects → auto-mute + open Slack |

## Phase 6 — Enterprise & Fleet (📋 Stubbed)

| Feature | Crate | Description |
|---------|-------|-------------|
| Central Policy Engine | `phase6::StubAdmin` | Remote configuration and compliance |
| LDAP/SSO | `phase6::StubAdmin` | Enterprise authentication integration |
| Audit Logging | `phase6::StubAdmin` | Zero-knowledge encrypted audit trail |
| Fleet Management | `phase6::StubFleet` | Fleet console and remote deployment |

## Implementation Notes

### Adding a new Phase 1-6 service

1. Create a new crate under `crates/<name>/`
2. Implement the `Service` trait (from `crates/services/src/traits.rs`)
3. Register it in `runtime/daemon/src/main.rs` via `registry.register(Box::new(...))`
4. Add packet routing in `crates/services/src/registry.rs::route_packet()` if needed
5. Add the crate to workspace members in `Cargo.toml`

### Extending the protocol

New message types go in `crates/protocol/src/message.rs`:
```rust
pub enum MessageType {
    // ... existing types ...
    NewFeature = 10,
}
```

### Platform-specific backends

Feature crates hide the OS behind a `platform` module:

- `crates/clipboard/src/platform.rs` — clipboard read/write via `arboard`
- `crates/input/src/platform.rs` — input injection via `enigo`

`arboard` and `enigo` already support Windows, macOS, and Linux, so all three
platforms share the same service code. Extending a platform means adding a
backend inside one of these modules, not touching the daemon or the protocol.

## Dependency Graph / Phase Ordering

```
Phase 1 ─────────────────────► Phase 2 ──────────► Phase 4 ──► Phase 6
    │                              │                    │
    │                              ▼                    │
    └────────────────────────► Phase 3                  │
                                   │                    │
                                   └──► Phase 5 ────────┘
```

Phases 2-6 are designed to be developed in parallel, as each builds on
the Phase 1 foundation (discovery, transport, sessions) through the
service abstraction layer.
