# UniKey

**OS-layer workspace mesh** — a cross-platform project for connecting devices
into a shared workspace. The current MVP has been verified on macOS; Windows,
Linux, Android, and iOS remain targets until tested on those platforms.

> **MVP status (2026-10-21):** Two isolated daemon instances have been verified
> end to end on macOS: UDP discovery, authenticated encrypted transport, PIN
> pairing with identity-key pinning, and SHA-256-verified file transfer. See
> [the MVP audit and checklist](docs/MVP_STATUS.md) for exact evidence and known
> limitations.

UniKey connects all your devices into a shared workspace with:

- 🔗 **Universal Clipboard (experimental)** — implemented and unit-tested, but
  not yet verified between physical devices
- 🖱️ **Universal Input (experimental)** — implemented and unit-tested; real OS
  permissions and cross-device behaviour remain unverified
- 📁 **P2P File Transfer (MVP)** — verified between two local daemon instances
- 🎯 **Edge Switching (planned demo work)** — not part of the verified MVP path
- 🔐 **E2E Encrypted** — Ed25519 identity, X25519 key agreement, ChaCha20-Poly1305
  sessions, replay protection, zero mandatory cloud dependencies

The core is a **Rust daemon** (`daemon`) that handles discovery, transport,
pairing, and services, plus an **HTTP API** on localhost that the desktop UI
(Tauri + React shell) talks to.

```
┌──────────────────────────────────────────────────────────────┐
│                    UniKey Daemon (Rust)                      │
│  ┌──────────┐ ┌──────────┐ ┌─────────┐ ┌─────────────────┐  │
│  │Discovery │ │Transport │ │ Pairing │ │  Services       │  │
│  │  (UDP)   │ │ (TCP)    │ │ (PIN/QR)│ │  Clipboard,     │  │
│  └──────────┘ └──────────┘ └─────────┘ │  Input, Files,   │  │
│                                        │  Phases 2–6 stubs │ │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Protocol (binary codec + messages)                  │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Runtime bootstrap: paths · config · logging · API   │   │
│  └──────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────┘
```

---

## Highlights

- **Installable or portable** — Windows users get a current-user NSIS desktop
  installer; engineering builds also provide a self-contained daemon folder.
  Neither requires Rust, Cargo, Node.js, Git, or Visual Studio at runtime.
- **One-command builds** — `./scripts/build.sh` compiles, strips, packages,
  zips, and prints a summary.
- **Self-healing runtime** — config, logs, keys, and identity are all
  auto-created with safe defaults. The daemon never crashes because a file is
  missing.
- **Cross-platform architecture** — macOS, Linux, and Windows share the same
  business crates. Only macOS has been exercised during the current MVP audit.
- **Native CI packaging** — GitHub Actions builds the Windows installer on a
  genuine Windows runner and smoke-tests install, packaged-daemon startup,
  graceful exit, and uninstall. Physical cross-device Windows behavior still
  requires the separate laptop checklist.

---

## Quick start (running the release)

```bash
# Build a portable release (macOS/Linux)
./scripts/build.sh --clean

# Copy the release folder to any machine and run the daemon:
release/UniKey/daemon          # or daemon.exe on Windows
```

Double-click `daemon` (or `daemon.exe`) and the daemon starts silently.
Confirm it is running:

```bash
curl http://127.0.0.1:45895/api/v1/health
# {"success":true,"data":{"status":"running","version":"0.1.0", ...}}
```

Everything the daemon needs is created next to the executable — config,
logs, keys, device identity — because the release ships a `unikey.portable`
marker.

---

## Building from source

### Prerequisites

- **Rust** 1.85+ (edition 2024). Install from <https://rustup.rs>.
- **git** (used to stamp builds with the commit hash; optional).
- **zip** (used by the bash packaging step; optional — the folder is always
  produced even when zipping is skipped).
- For the desktop UI: **Node.js 22+** and npm (React + Vite + Tauri v2). The
  shell can also be built without one — it runs the daemon headless either way.

### Commands

```bash
# Build all crates (debug)
cargo build --workspace

# Run the daemon in dev mode (standard per-user config, console logging)
cargo run --release -p daemon

# Run the test suite (well under a minute)
./scripts/test.sh

# Full release build + portable folder + zip
./scripts/build.sh --clean

# Native desktop bundle with a freshly staged release daemon
./scripts/build.sh --ui --no-zip

# Same on Windows (PowerShell)
.\scripts\build.ps1 -Clean -UI
```

See [docs/BUILD.md](docs/BUILD.md) for a deeper walkthrough and
[docs/RELEASE.md](docs/RELEASE.md) for the release checklist.
For the physical macOS-to-Windows verification procedure, see
[docs/WINDOWS_TESTING.md](docs/WINDOWS_TESTING.md).
For installer architecture, permissions, CI smoke testing, and signing status,
see [docs/WINDOWS_RELEASE.md](docs/WINDOWS_RELEASE.md).

Use the repository build script for native packaging. A direct
`npx tauri build` embeds whichever `src-tauri/unikey-daemon.bin` is already
staged and does not rebuild that daemon automatically.

---

## Project structure

```
.
├── crates/                 # Modular Rust library crates
│   ├── common/             #   constants, version, error types
│   ├── device/             #   device identity, keypair, repository
│   ├── protocol/           #   binary protocol: messages, hello, frames
│   ├── crypto/             #   keys, AEAD, KDF, replay, hashing
│   ├── discovery/          #   UDP discovery + peer manager
│   ├── transport/          #   TCP connections + encrypted sessions
│   ├── services/           #   service trait + registry
│   ├── pairing/            #   pairing sessions + trust store
│   ├── events/             #   event bus + typed live events
│   ├── clipboard/ input/ files/   # feature services
├── runtime/
│   └── daemon/             # The binary: bootstrap, config, logging, API
├── apps/
│   └── desktop/            # Tauri v2 desktop shell (React frontend)
├── scripts/                # Build/test/release tooling (bash + PowerShell)
├── config/                 # Default shipped configuration (unikey.toml)
├── assets/                 # Icons, fonts, themes, locale (shipped verbatim)
├── docs/                   # Architecture, build, release, troubleshooting
└── release/                # Generated release output (git-ignored)
```

### Runtime layout

The daemon supports two layouts, detected by the presence of
`unikey.portable` next to the executable:

```text
release/UniKey/                 ← portable (state lives here)
├── unikey.portable             ← marker file
├── daemon(.exe)                ← self-contained binary
├── config/                     ← unikey.toml, device.json, trusted.json, keys/
├── logs/                       ← latest.log, rotated backups, panic.log
├── assets/                     ← icons, fonts, themes, locale
├── README.txt · LICENSE · version.json
```

Without the marker, the daemon uses the OS per-user config directory
(`~/Library/Application Support`, `%APPDATA%`, or `~/.config`) under
`UniKey/`, preserving the historical layout.

---

## Testing

```bash
./scripts/test.sh            # or: cargo test --workspace

# Real two-daemon MVP journey on one host. Uses isolated temporary state and
# real UDP/TCP sockets; it does not mock discovery, pairing, or file transfer.
./scripts/mvp-e2e.sh
```

The suite covers the runtime infrastructure (path resolution, config
defaults, log rotation, build metadata) plus existing crate logic. It is
designed to finish in **under a minute**.

The end-to-end script requires the test discovery, TCP, and API ports to be
available. It validates discovery, an authenticated encrypted session, PIN
pairing, pinned trust, file transfer, and received-file content.

---

## Releasing

```bash
# macOS / Linux
./scripts/build.sh --clean

# Windows (PowerShell)
.\scripts\build.ps1 -Clean -UI
```

Produce:

```text
release/UniKey/                 ← copy anywhere, double-click to run
release/UniKey-v0.1.0-<OS>.zip  ← distributable archive
release/UniKey-Setup.exe        ← recommended Windows desktop installer
```

`version.json` inside the release is stamped with the version, build date,
git commit, and platform — ready for the future auto-updater.
See [docs/RELEASE.md](docs/RELEASE.md).

---

## Troubleshooting & FAQ

Having a problem? Start with [docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md),
which covers common errors, Windows runtime notes, and a FAQ. Quick answers:

- **The daemon won't start** → check `logs/latest.log` and `logs/panic.log` in
  the release folder; ports may be in use.
- **I need a different port** → edit `config/unikey.toml` and restart.
- **Where is my data?** → In portable mode, inside the release folder. In dev
  mode, under the OS config directory.
- **Windows binary needs a DLL?** → It doesn't. UniKey statically links the
  MSVC CRT (see `docs/TROUBLESHOOTING.md`).

---

## License

MIT — see [LICENSE](LICENSE).
