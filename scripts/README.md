# `scripts/` — build, test, release and packaging tooling

Every script is self-contained, documented, and fails with a readable message
rather than a bare error. All paths are derived from the script location —
nothing is hardcoded.

| Script                            | Purpose                                                        |
|-----------------------------------|----------------------------------------------------------------|
| `build.sh` / `build.ps1`          | Full build + portable release folder + `.zip` archive         |
| `clean.sh` / `clean.ps1`          | Remove build output (`--all` for the whole `target/`)         |
| `test.sh` / `test.ps1`            | Run the workspace test suite (`--release` optional)           |
| `gen-version.sh` / `gen-version.ps1` | Write `version.json` for the current source tree           |
| `build/common.sh` / `common.ps1`  | Shared helpers (sourced, not run)                             |
| `templates/README.txt`            | Release-folder README, filled with version/platform data      |

## Build (one command)

```bash
# macOS / Linux
./scripts/build.sh --clean

# Windows (PowerShell)
.\scripts\build.ps1 -Clean
```

Output:

```text
release/
├── UniKey/                     <- the portable application folder
│   ├── unikey.portable         <- marker: daemon keeps all state in-folder
│   ├── daemon(.exe)            <- self-contained daemon binary
│   ├── assets/                 <- copied verbatim from repo `assets/`
│   ├── config/unikey.toml      <- editable defaults
│   ├── logs/                   <- created at first run
│   ├── README.txt              <- generated from the template
│   ├── LICENSE
│   └── version.json            <- version / commit / date / platform
└── UniKey-v0.1.0-<OS>.zip      <- distributable archive
```

Copy `release/UniKey/` (or unzip the archive) to another machine and run
`daemon` — no Rust, no Cargo, no Visual Studio, no install.

## What each script does

### `build.sh` / `build.ps1`

1. (optional `--clean`) wipe `target/release` and `release/`.
2. `cargo build --release -p daemon` — the release profile in `Cargo.toml`
   strips symbols and optimises for size automatically.
3. Assemble `release/UniKey/`: binaries, `assets/`, `config/unikey.toml`,
   `README.txt`, `LICENSE`, `version.json`, and the `unikey.portable` marker.
4. Zip the folder to `release/UniKey-v<version>-<OS>.zip`.
5. Print a summary: platform, profile, version, build time, binary size,
   package size, and the output path.

Flags: `--clean`, `--ui` (experimental desktop UI), `--no-zip`, `--profile`.

### `clean.sh` / `clean.ps1`

Remove `release/` and (optionally, `--all`) `target/`. Run before release
builds for a byte-for-byte fresh binary.

### `test.sh` / `test.ps1`

`cargo test --workspace`. The suite is intentionally small and fast — it
completes in well under a minute.

### `gen-version.sh` / `gen-version.ps1`

Writes `version.json` (version from `Cargo.toml`, commit from `git`, UTC build
date, platform) to stdout or a supplied path. Used by CI to stamp releases
without running a full build.

## Error handling

| Condition            | Behaviour                                                          |
|----------------------|--------------------------------------------------------------------|
| `cargo` missing      | readable message, exit 1                                           |
| build fails          | "Compilation failed" + pointer to `docs/TROUBLESHOOTING.md`        |
| `assets/` missing    | warning; build continues without assets                            |
| `config/unikey.toml` missing | warning; daemon generates defaults on first run              |
| `zip` missing (bash) | warning; folder is produced, archive is skipped                    |
| permission denied    | propagated by the shell with the failing path                      |

## Adding a script

Keep the platform pair (`.sh` + `.ps1`) in sync. Put shared logic in
`build/common.sh` / `common.ps1`, keep the entry points thin, and update this
table.
