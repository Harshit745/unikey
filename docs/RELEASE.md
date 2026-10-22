# Releasing UniKey

## The release contract

Every release is a **portable folder** whose contract is:

1. **Self-contained** — it runs on a target machine with no Rust, Cargo,
   Visual Studio, Git, or installers.
2. **State-in-folder** — while `unikey.portable` is present, the daemon reads
   and writes everything inside the folder: config, logs, keys, identity.
3. **Versioned** — `version.json` at the folder root describes the build.
4. **Idempotent to replace** — upgrading means swapping the folder contents;
   user state survives as long as `config/` is preserved.

## Producing a release

```bash
# macOS / Linux daemon-only portable folder
./scripts/build.sh --clean

# macOS desktop application bundle
./scripts/build.sh --clean --ui

# Windows daemon-only portable folder
.\scripts\build.ps1 -Clean

# Windows desktop installer
.\scripts\build.ps1 -Clean -UI
```

Output:

```text
release/
├── UniKey/                       ← the portable application folder
│   ├── unikey.portable
│   ├── daemon(.exe)
│   ├── assets/  config/  logs/
│   ├── README.txt  LICENSE  version.json
└── UniKey-v0.1.0-<OS>.zip
```

## Versioning

- The single source of truth is `[workspace.package] version` in `Cargo.toml`
  (currently `0.1.0`).
- `version.json` is generated twice: by the build script (canonical layout)
  and by the daemon at first run (richer, includes `profile` and `rustc`).
  Either satisfies tooling that reads `version`, `build`, `commit`,
  `platform`.

```json
{
  "version": "0.1.0",
  "build": "2026-08-07",
  "commit": "511bb465d059",
  "platform": "macOS"
}
```

## Release checklist

1. Bump `version` in `Cargo.toml` (workspace package).
2. `./scripts/test.sh` — suite green.
3. `./scripts/build.sh --clean` — clean release folder + zip.
4. Copy `release/UniKey/` to a **clean** machine and run `daemon`.
5. Confirm `http://127.0.0.1:45895/api/v1/health` reports the new version.
6. Complete the physical-device steps in `WINDOWS_TESTING.md` and record the
   tested operating-system versions and hashes.
7. Push the commit and wait for both GitHub Actions workflows to pass.
8. Tag the exact tested commit (`git tag -a v0.1.0 -m "UniKey v0.1.0"`) and
   push that tag. CI rebuilds the native installers and creates the Release.

## Publishing to GitHub

Authenticate GitHub CLI using its browser flow; never paste a token into a
tracked file or shell script:

```bash
gh auth login --hostname github.com --web --git-protocol https
gh auth setup-git --hostname github.com
gh auth status --hostname github.com
```

After reviewing the exact staged files and committing them:

```bash
git push origin main
# Wait for the main-branch workflows to pass before creating a release tag.
git tag -a v0.1.0 -m "UniKey v0.1.0"
git push origin v0.1.0
```

The tag starts the desktop release job. Its GitHub Release contains stable,
user-facing filenames:

- `UniKey-macOS-universal.dmg`
- `UniKey-Windows-x64-Setup.exe`

Ordinary branch builds remain downloadable from the workflow run's **Artifacts**
section for 30 days. Tagged Release assets remain available from the
repository's **Releases** page.

## CI / CD

`.github/workflows/build.yml` builds Windows, macOS, and Linux daemon bundles
from the same workspace, runs the tests, and uploads each platform's zip as a
workflow artifact.

`.github/workflows/desktop.yml` additionally builds the **Tauri desktop shell**
(embedding the daemon binary) into native bundles for macOS (`universal`
DMG) and Windows (NSIS installer), and runs a smoke test that starts the daemon
and probes its health/version/services/peers endpoints. A `v*` tag publishes
the DMG and Windows installer to one GitHub Release.

## Auto-updater preparation

Updates are **not implemented** — the structure is ready for them:

- The updater would download a new `UniKey-v<version>-<OS>.zip` from a release
  feed, verify it against `version.json`, unpack it into a sibling folder,
  swap binaries, and restart. Rollback = keep the previous folder.
- `version.json` is deliberately small, stable, and at a fixed path
  (`<release>/version.json`) so an updater can parse it without knowing where
  mutable config is stored.
- The daemon writes `version.json` itself at startup, so a hand-assembled
  release folder is still self-describing.

Design notes for when the updater lands:

- Compare `version` (semver) — never `build` (a date) — for upgrade decisions.
- Preserve `config/` across swaps; only replace binaries and assets.
- Validate the downloaded archive's `version.json` before applying.
- Keep a `previous/` folder for rollback.
