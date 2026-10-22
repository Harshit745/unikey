# Troubleshooting UniKey

A practical guide to diagnosing and fixing common problems — from build
failures to runtime crashes.

## Where to look first

| Symptom                      | First stop                                              |
|------------------------------|---------------------------------------------------------|
| Build fails                  | `cargo build --release -p daemon --verbose`             |
| Daemon won't start           | `logs/latest.log` (and `logs/panic.log`) in the data root |
| Daemon crashes               | `logs/panic.log`                                        |
| Port conflict                | `config/unikey.toml` → `[network]`                      |
| UI can't reach the daemon    | `curl http://127.0.0.1:45895/api/v1/health`             |

## Build errors

### `cargo: command not found`
Rust is not installed or not on `PATH`. Install from <https://rustup.rs>, then
`source "$HOME/.cargo/env"` (or reopen the shell).

### Compilation fails
Re-run with full output:

```bash
cargo build --release -p daemon --verbose 2>&1 | tee build.log
```

Common causes:
- Wrong Rust version (the workspace declares `rust-version = "1.85"`;
  `rustc --version` should be ≥ that).
- A crate failed its own build script — the specific crate name is in the
  error. Try `cargo clean` first; stale fingerprints are the usual culprit.

### `zip` is missing (Linux)
The release folder is still produced; only the archive is skipped. Install:
`apt install zip` (Debian/Ubuntu) or `dnf install zip` (Fedora).

### Release binary is large
The `release` profile already uses `opt-level = "z"`, `lto = true`,
`codegen-units = 1`, and `strip = true`. If it is still large, the UI bundle
(`--ui`) dominates; the daemon binary itself is ~1–2 MB.

## Runtime errors

### Daemon starts then exits immediately
Check `logs/latest.log` first, then `logs/panic.log`. The most common cause is
a port already in use:

```text
[TCP] Failed to bind 0.0.0.0:45892: Address already in use
```

Fix: change the port in `config/unikey.toml` (`tcp_port`, `discovery_port`,
`api_port`) and restart. Ports must be unique; `0` disables a listener.

### "Failed to bind" on the API port
`api_port` (default 45895) collides. Change it in `config/unikey.toml`. If you
change it, the desktop UI must use the same port.

### Panic / crash — `logs/panic.log`
The daemon appends a crash report to `logs/panic.log` before exiting:

```text
==== PANIC 2026-08-07T09:51:57Z ====
Message: <panic payload>
Location: <file>:<line>:<col>
```

Report the `Message` and `Location` when filing a bug. With `panic = "abort"`
(the release profile) there is no unwind backtrace, but the location and
payload are always captured.

## Windows-specific

### "The code execution cannot proceed because VCRUNTIME140.dll was not found"
Should not happen. UniKey statically links the MSVC CRT for Windows via
`.cargo/config.toml`:

```toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-Ctarget-feature=+crt-static"]
```

That flag links `vcruntime`/`ucrt` into the binary, so the release runs on any
Windows 10+ machine with zero installers. If you ever see this error, the
binary was built without the flag (e.g. on the GNU toolchain) — rebuild with
the MSVC target or install the VC++ redistributable.

### The daemon shows no window
By design. Release builds use `windows_subsystem = "windows"` (see
`runtime/daemon/src/main.rs`), so the daemon runs without a console. It logs
to `logs/latest.log`. Debug builds keep the console for development.

### Double-clicking `daemon.exe` does nothing visible
It is likely running — check `logs/latest.log` and the health endpoint.
To stop it, use Task Manager (end `daemon.exe`).

### The installed desktop app says the daemon is offline

Check `%APPDATA%\UniKey\logs\latest.log`, then confirm that
`http://127.0.0.1:45895/health` returns `{ "status": "ok" }`. The packaged
daemon is extracted under `%LOCALAPPDATA%\UniKey\runtime`. Do not add a
firewall rule for 45895: it is deliberately localhost-only. If an older
standalone daemon is already using the port, stop that process and use the
desktop app's **Retry** action.

### Windows warns that the installer is unrecognized

Development installers are currently unsigned. Windows SmartScreen may warn
about a build that has not established publisher reputation. Verify the
SHA-256 file from the same CI artifact. Public distribution requires a
reviewed code-signing certificate; do not disable SmartScreen system-wide.

## macOS-specific

### "Cannot open daemon because it is from an unidentified developer"
The binary is unsigned. Right-click → **Open** → **Open**, or run
`xattr -d com.apple.quarantine release/UniKey/daemon` to clear the quarantine
attribute. For distribution, sign and notarise the binary (future work).

## FAQ

**Where is my data stored?**
Portable releases: inside the release folder (`config/`, `logs/`). Installed
Windows and development runs: the OS per-user config directory under
`UniKey/` (`%APPDATA%\UniKey` on Windows).

**How do I change the device name?**
Set `device_name` in `config/unikey.toml` and restart the daemon.

**How do I rotate logs manually?**
You don't need to — `latest.log` rotates automatically at `max_bytes`
(5 MiB by default) and keeps `max_backups` (5) backups.

**The UI is missing.**
The portable release ships the headless daemon. To build the Tauri desktop
shell, run `./scripts/build.sh --ui` (or `.\scripts\build.ps1 -UI`), which
stages the daemon for embedding and runs `npm ci && tauri build`. The daemon
(and its HTTP API) is fully functional on its own either way.

**How do I update the ports across my fleet?**
Each machine has its own `config/unikey.toml`. Ports are not centrally
managed yet — consider shipping a config via the release pipeline if you need
a fleet default.
