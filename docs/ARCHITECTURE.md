# UniKey Architecture

## Design Principles

1. **Local-first** — No cloud dependency. All communication is direct peer-to-peer over the local network.
2. **E2E encrypted** — Nothing beyond discovery travels in plaintext. Every session is authenticated (Ed25519) and encrypted (X25519 + ChaCha20-Poly1305).
3. **Layered** — The GUI never owns sockets. UI ↓ HTTP API ↓ daemon ↓ session manager ↓ encrypted transport ↓ services.
4. **Modular services** — Every feature is a `Service` impl in a self-contained crate, registered with the daemon at startup.
5. **Never trust unknown devices** — Unknown devices can discover you, but nothing else until a pairing completes.

## System Overview

```
┌────────────────────────────────────────────────────────────────┐
│  Desktop GUI — Tauri v2 shell (apps/desktop)                    │
│  React + Vite + TypeScript  ·  system tray  ·  notifications    │
└───────────────────────────────┬────────────────────────────────┘
                                │  HTTP/JSON on 127.0.0.1:45895
┌───────────────────────────────▼────────────────────────────────┐
│  Daemon — runtime/daemon (the only networked process)            │
│                                                                │
│  ┌──────────────┐   ┌───────────────┐   ┌────────────────────┐  │
│  │ HTTP API      │   │ Service       │   │ Session Manager    │  │
│  │ (axum, local) │   │ Registry      │   │ (encrypted peers)  │  │
│  └──────┬───────┘   └───────┬───────┘   └─────────┬──────────┘  │
│         │                  │                      │             │
│  ┌──────▼───────┐   ┌──────▼───────┐   ┌──────────▼──────────┐  │
│  │ EventBus     │◄──┤ Clipboard /  │◄──┤ Transport           │  │
│  │ (SSE → GUI)  │   │ Input / Files│   │ (TCP, handshake,    │  │
│  └──────────────┘   └──────────────┘   │  encrypted frames)  │  │
│                                        └──────────┬──────────┘  │
│  ┌──────────────┐   ┌──────────────┐              │             │
│  │ Discovery    │   │ Pairing      │◄─────────────┘             │
│  │ (UDP beacons)│   │ (trust store)│                            │
│  └──────────────┘   └──────────────┘                            │
└─────────────────────────────────────────────────────────────────┘
```

- **The daemon is the only process that touches sockets.** It owns the UDP discovery socket, the TCP transport listener, and the localhost HTTP API. The Tauri shell talks to it exclusively over `127.0.0.1:45895`.
- **The daemon has no UI** (except an optional console). It runs headless in the background; the desktop shell and any HTTP client interact with it over the local API.

## Workspace Crate Layout

```
crates/
├── common/       Shared constants (protocol version, ports) and helpers
├── crypto/       Identity keys (Ed25519), X25519, ChaCha20-Poly1305 AEAD,
│                 HKDF key derivation, SHA-256, replay guard, key rotation
├── device/       Device identity management (load/create, persisted key paths)
├── protocol/     Message types, hello handshake, encrypted frame codec
├── discovery/    UDP beacons + peer registry (RTT, online/offline, pruning)
├── transport/    TCP connection manager, session manager, handshake,
│                 encrypted framing, heartbeat, reconnect, idle reaping
├── pairing/      PIN/QR pairing flow, Ed25519 trust exchange, persistent
│                 trust store, pairing history
├── events/       EventBus (pub/sub broadcast) + typed AppEvent envelope
├── services/     Service trait + registry used to wire features together
├── clipboard/    Clipboard sync (arboard), loop prevention, text/HTML/image
├── input/        Keyboard & mouse forwarding (enigo), cursor grab/release
└── files/        Chunked file transfer: metadata, chunks, SHA-256 verify,
                  resume/pause/cancel, progress, transfer manager

runtime/
└── daemon/       The binary: wires everything together, HTTP API, logging,
                  config, paths, build info, graceful shutdown

apps/
└── desktop/      Tauri v2 shell: React frontend + Rust host (spawns the
                  daemon, system tray, native notifications, SSE watcher)
```

Dependency direction is strictly downward: `services` depends on `transport`, `protocol`, and `events`; nothing depends on `services`.

## Daemon Startup Lifecycle

1. **Resolve paths** — config dir, keys dir, trust store, logs, downloads.
2. **Load config** (`unikey.toml`) — missing/malformed files fall back to defaults, never crash.
3. **Device identity** — load or create the persisted Ed25519 keypair + `DeviceIdentity`.
4. **Load trust store** — the set of previously-paired devices.
5. **Create the EventBus** and hand it to the session manager (single lifecycle chokepoint).
6. **Register services** (clipboard, input, files, pairing) — each gets a context with session manager + shutdown token.
7. **Start the HTTP API** on `127.0.0.1:45895`.
8. **Start the TCP transport** listener (port 45892) for inbound connections.
9. **Start UDP discovery** (port 45893) — broadcast beacons every 2 s, record peers, auto-connect to known ones.
10. **Await shutdown** — Ctrl+C / tray Quit cancels the token; services stop, sockets close.

Non-fatal failures degrade gracefully: if the discovery socket can't bind, the daemon still serves the API and TCP transport — it just runs without LAN discovery.

## Data Flow

### Discovery → Session
```
Device A                     LAN                        Device B
   │ UDP beacon (every 2s) ──▶│── UDP beacon ──▶            │
   │◀─── UDP reply ──────────│◀── UDP reply ──│             │
   │                          │                 │           │
   │── TCP connect + Hello ──▶│── TCP connect + Hello ──▶    │   (lower peer id
   │◀── Hello ────────────────│◀── Hello ──────────────────│   wins the dial)
   │── Session Established ───│── Session Established ──────│
```

### Packet → Frame → Service
```
TCP stream
   │  split on 4-byte length prefix
   ▼
EncryptedFrame::decode()        (transport/connection.rs)
   │  replay check → AEAD open → payload is a Packet
   ▼
Packet { version, type, payload } dispatch
   ├── Hello/Disconnect        → ConnectionManager
   ├── Ping/Pong               → transport heartbeat
   ├── PairRequest/PairResponse→ PairingService
   ├── Clipboard               → ClipboardService
   ├── FileMetadata/Chunk/Ack  → FileTransferService
   ├── FileCancel              → FileTransferService
   └── Input* / InputGrab      → InputService
```

## Event System

Services publish typed events to a shared `EventBus` (broadcast channel, bounded history). The GUI consumes them two ways:

- **HTTP API** — `GET /api/v1/events` is a Server-Sent Events stream of live events (session up/down, clipboard synced, transfer progress, pairing completed).
- **Desktop notifications** — the Tauri shell opens a raw SSE connection and maps interesting events to native OS notifications. Routine events (clipboard sync, progress) are silent.

Every event is an `Envelope { timestamp, event }` where `event` is `AppEvent` serialized with a `type` discriminator (e.g. `"session_connected"`, `"transfer_completed"`).

## Config

The daemon reads `unikey.toml` (schema version 1.0). Sections:

| Section | Purpose |
|---------|---------|
| `daemon`   | Device display-name override |
| `network`  | TCP / UDP / API ports, bind addresses |
| `logging`  | Level, rotation size, backup count, file enable |
| `services` | Per-feature toggles (clipboard, input, files, pairing) |
| `clipboard` | Auto-sync, images, history limit, poll interval |
| `input`    | Keyboard/mouse forwarding, edge switching |
| `transfer` | Max file size (0 → protocol max), download dir |
| `encryption` | Key rotation interval, replay window, pairing required |

New sections carry `#[serde(default)]`, so config files written by older versions keep loading. `PUT /api/v1/config` persists a validated update and live-swaps it without a restart.

## Performance Targets

| Metric | Target |
|--------|--------|
| Idle RAM | < 100 MB |
| Idle CPU | < 1% |
| Mouse/key latency (LAN) | < 5 ms |
| Clipboard sync | < 100 ms |
| Startup to healthy API | < 500 ms |

## See Also

- [NETWORKING.md](NETWORKING.md) — wire protocol, discovery, transport details
- [ENCRYPTION.md](ENCRYPTION.md) — crypto design and guarantees
- [API.md](API.md) — HTTP API reference
- [DEVELOPER.md](DEVELOPER.md) — working on the codebase
- [SECURITY.md](SECURITY.md) — threat model
