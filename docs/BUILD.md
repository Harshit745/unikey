# Building UniKey

This page explains the build pipeline in detail: what runs, what is produced,
and how to customise it.

## One-command build

```bash
# macOS / Linux
./scripts/build.sh --clean

# Windows (PowerShell)
.\scripts\build.ps1 -Clean

# Windows NSIS desktop installer
.\scripts\build.ps1 -Clean -UI
```

The script does, in order:

1. **Clean** — removes `target/release` and `release/` (flags: `--clean` /
   `-Clean`).
2. **Compile** — `cargo build --release -p daemon`.
3. **Assemble** — builds `release/UniKey/`:
   - binaries (`daemon` / `daemon.exe`)
   - `assets/` copied verbatim from the repo
   - `config/unikey.toml` (default configuration)
   - `README.txt` (generated from `scripts/templates/README.txt`)
   - `LICENSE`
   - `version.json` (version, commit, date, platform)
   - `unikey.portable` marker → the daemon keeps all state in-folder
4. **Zip** — `release/UniKey-v<version>-<OS>.zip` (skippable with `--no-zip`).
5. **Report** — platform, profile, version, build time, binary size, package
   size, output path.

With `-UI` on Windows, the script additionally builds the Tauri shell, embeds
the freshly compiled Windows daemon, and copies the NSIS package to
`release\UniKey-Setup.exe`. See [WINDOWS_RELEASE.md](WINDOWS_RELEASE.md) for
installer behavior, permissions, CI verification, and signing status.

## Build flags

| Flag (bash)  | Switch (PowerShell) | Effect                                    |
|--------------|---------------------|-------------------------------------------|
| `--clean`    | `-Clean`            | wipe previous build output first          |
| `--ui`       | `-UI`               | also build the Tauri desktop shell (stages the daemon for embedding, runs `npm ci && tauri build`) |
| `--no-zip`   | `-NoZip`            | produce the folder but skip the archive   |
| `--profile=` | `-Profile`          | build profile (default `release`)         |

## Release profile

`Cargo.toml` tunes the `release` profile for small, portable binaries:

```toml
[profile.release]
opt-level = "z"        # optimise for size
lto = true             # fat link-time optimisation
codegen-units = 1
panic = "abort"        # smaller binary; the daemon still logs crashes first
strip = true           # remove symbol tables / debuginfo
```

Notes:

- `strip = true` makes the extra `strip` step in old build scripts
  unnecessary — the binary leaves the compiler already stripped.
- `panic = "abort"` trades unwinding for size. The daemon installs a panic
  hook that writes `logs/panic.log` **before** aborting, so crash diagnostics
  are preserved. Tests that rely on `#[should_panic]` must run under the dev
  profile (the default for `cargo test`).

## Version stamping

`runtime/daemon/build.rs` captures, at compile time:

| Variable            | Source                                     | Fallback   |
|---------------------|--------------------------------------------|------------|
| `UNIKEY_COMMIT`     | `git rev-parse --short=12 HEAD`            | `unknown`  |
| `UNIKEY_BUILD_EPOCH`| `SOURCE_DATE_EPOCH` (CI) or wall clock     | `0`        |

These are exposed at runtime via `build_info::BuildInfo` and written to
`version.json` in the release folder. CI can pin `SOURCE_DATE_EPOCH` for
reproducible builds and `UNIKEY_OVERRIDE_COMMIT` for tag-based builds.

## Running in development

```bash
cargo run --release -p daemon
```

Without a `unikey.portable` marker next to the binary, the daemon uses the OS
config directory and prints logs to the console — the same behaviour as
before this pipeline existed.

## Common build problems

- **`cargo: command not found`** — Rust is not installed or not on `PATH`.
  Install from <https://rustup.rs>.
- **Compilation fails** — re-run with `--verbose`; see
  [TROUBLESHOOTING.md](TROUBLESHOOTING.md).
- **`zip` missing on Linux** — the folder is still produced; install with
  `apt install zip` (Debian/Ubuntu) or `dnf install zip` (Fedora) to get the
  archive too.
