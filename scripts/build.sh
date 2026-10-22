#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# UniKey — build & package (macOS / Linux)
#
#   ./scripts/build.sh [--clean] [--ui] [--no-zip] [--profile release]
#
# Flags:
#   --clean    wipe target/release and the release/ folder before building
#   --ui       also build the Tauri desktop shell and bundle it (requires npm)
#   --no-zip   build the release folder but skip the .zip archive
#   --profile  build profile (default: release)
#
# Produces:
#   release/UniKey/                       portable application folder
#   release/UniKey-v<ver>-<OS>.zip        distributable archive
#
# Fails gracefully with a readable message when cargo, git, assets, or the
# config are missing, when the build fails, or on permission problems.
# ─────────────────────────────────────────────────────────────────────────────
set -Eeuo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export REPO_ROOT
PLATFORM="unknown"

case "$(uname -s 2>/dev/null || true)" in
  Darwin) PLATFORM="osx" ;;
  Linux)  PLATFORM="linux" ;;
  MINGW*|MSYS*|CYGWIN*) PLATFORM="windows" ;;
esac

# ─── Flags ─────────────────────────────────────────────────────────────────
CLEAN=0
BUILD_UI=0
MAKE_ZIP=1
PROFILE="release"
for arg in "$@"; do
  case "$arg" in
    --clean)  CLEAN=1 ;;
    --ui)     BUILD_UI=1 ;;
    --no-zip) MAKE_ZIP=0 ;;
    --profile=*) PROFILE="${arg#*=}" ;;
    --help|-h)
      grep '^#' "$0" | sed 's/^# \{0,1\}//'
      exit 0 ;;
    *) printf '%s\n' "Unknown option: $arg (see --help)" >&2; exit 2 ;;
  esac
done

# shellcheck source=build/common.sh
source "$REPO_ROOT/scripts/build/common.sh"

require_cmd "cargo" "install Rust from https://rustup.rs (or set PATH)."
require_cmd "mkdir" "this should never happen."

# ─── Metadata ───────────────────────────────────────────────────────────────
VERSION="$(version_from_cargo)"
PLATFORM_TITLE="$(platform_title)"
COMMIT_SHORT="$(git_commit)"
BUILD_DATE="$(build_date)"

# Release layout
RELEASE_ROOT="$REPO_ROOT/release"
RELEASE_DIR="$RELEASE_ROOT/UniKey"
ZIP_NAME="UniKey-v${VERSION}-${PLATFORM_TITLE}.zip"
ZIP_PATH="$RELEASE_ROOT/$ZIP_NAME"

BIN_NAME="daemon"
[ "$PLATFORM" = "windows" ] && BIN_NAME="daemon.exe"
UI_NAME="UniKey"
[ "$PLATFORM" = "windows" ] && UI_NAME="UniKey.exe"

START_TIME="$(date +%s)"

# ─── Banner ─────────────────────────────────────────────────────────────────
printf '%s\n' "=============================="
printf '%s\n' "  ${C_BOLD}UniKey Build${C_OFF}"
printf '%s\n' "=============================="
info "Platform : $PLATFORM_TITLE"
info "Profile  : $PROFILE"
info "Version  : $VERSION"

# ─── Clean (optional) ───────────────────────────────────────────────────────
if [ "$CLEAN" -eq 1 ]; then
  info "Cleaning previous build artifacts…"
  cargo clean --manifest-path "$REPO_ROOT/Cargo.toml" -p daemon \
    || warn "cargo clean failed (continuing)."
fi
rm -rf "$RELEASE_ROOT"
mkdir_p "$RELEASE_ROOT"

# ─── Compile ────────────────────────────────────────────────────────────────
info "Compiling daemon (${PROFILE})…"
if ! cargo build --release --manifest-path "$REPO_ROOT/Cargo.toml" -p daemon; then
  fail "Compilation failed. Re-run with verbose output, or consult docs/TROUBLESHOOTING.md."
fi

# ─── Assemble release folder ────────────────────────────────────────────────
info "Assembling release folder…"
mkdir_p "$RELEASE_DIR/config"
mkdir_p "$RELEASE_DIR/logs"
mkdir_p "$RELEASE_DIR/assets"

# Portable marker — makes the daemon keep ALL state inside this folder.
printf '%s\n' "UniKey portable release. Removing this file switches the daemon to per-user config." \
  > "$RELEASE_DIR/unikey.portable"

# 1. Binaries
BIN_SRC="$REPO_ROOT/target/release/$BIN_NAME"
[ -f "$BIN_SRC" ] || fail "Expected built binary at target/release/$BIN_NAME — was the build clean?"
cp "$BIN_SRC" "$RELEASE_DIR/$BIN_NAME"
info "  - binary: $BIN_NAME ($(human_size "$(wc -c < "$RELEASE_DIR/$BIN_NAME" | tr -d ' ')"))"

# Optional Tauri desktop shell (only when --ui). Stages the freshly built
# daemon as `unikey-daemon.bin` so the shell embeds it, then runs `tauri build`.
if [ "$BUILD_UI" -eq 1 ]; then
  if command -v npm >/dev/null 2>&1; then
    info "Building Tauri desktop shell (this can take several minutes)…"
    cp "$BIN_SRC" "$REPO_ROOT/apps/desktop/src-tauri/unikey-daemon.bin"
    (cd "$REPO_ROOT/apps/desktop" && npm ci && npx tauri build) \
      || warn "UI build failed; continuing without the UI bundle."
    # Copy the native bundle(s) into the release folder best-effort.
    if [ -d "$REPO_ROOT/apps/desktop/src-tauri/target/release/bundle" ]; then
      cp -R "$REPO_ROOT/apps/desktop/src-tauri/target/release/bundle" "$RELEASE_DIR/UI"
      info "  - UI bundle copied"
    fi
  else
    warn "--ui requested but npm is not installed; continuing without the UI."
  fi
fi

# 2. Assets (whole directory — future assets ship automatically)
if [ -d "$REPO_ROOT/assets" ]; then
  cp -R "$REPO_ROOT/assets/." "$RELEASE_DIR/assets/"
  info "  - assets copied"
else
  warn "assets/ directory is missing at the repository root; shipping without assets."
fi

# 3. Configuration (default template; the daemon recreates it if removed)
if [ -f "$REPO_ROOT/config/unikey.toml" ]; then
  cp "$REPO_ROOT/config/unikey.toml" "$RELEASE_DIR/config/unikey.toml"
  info "  - config/unikey.toml copied"
else
  warn "config/unikey.toml is missing; the daemon will generate defaults on first run."
fi

# 4. README.txt (from template) + LICENSE
mkdir_p "$REPO_ROOT/scripts/templates"
if [ -f "$REPO_ROOT/scripts/templates/README.txt" ]; then
  fill_readme_template "$REPO_ROOT/scripts/templates/README.txt" "$RELEASE_DIR/README.txt"
  info "  - README.txt generated"
else
  warn "README.txt template missing; skipping."
fi
if [ -f "$REPO_ROOT/LICENSE" ]; then
  cp "$REPO_ROOT/LICENSE" "$RELEASE_DIR/LICENSE"
  info "  - LICENSE copied"
else
  warn "LICENSE file missing; skipping."
fi

# 5. version.json
gen_version_json "$RELEASE_DIR/version.json" "$VERSION" "$COMMIT_SHORT" "$BUILD_DATE" "$PLATFORM_TITLE"
info "  - version.json generated ($VERSION / $COMMIT_SHORT / $BUILD_DATE)"

# ─── Package ────────────────────────────────────────────────────────────────
if [ "$MAKE_ZIP" -eq 1 ]; then
  if command -v zip >/dev/null 2>&1; then
    info "Packaging ${ZIP_NAME}…"
    (cd "$RELEASE_ROOT" && zip -qr "$ZIP_NAME" UniKey)
    ZIP_BYTES="$(wc -c < "$ZIP_PATH" | tr -d ' ')"
  else
    warn "'zip' is not installed — skipped packaging. (Debian/Ubuntu: apt install zip)"
    ZIP_BYTES=0
  fi
else
  ZIP_BYTES=0
fi

# ─── Summary ─────────────────────────────────────────────────────────────────
BUILD_TIME="$(( $(date +%s) - START_TIME ))s"
BIN_BYTES="$(wc -c < "$RELEASE_DIR/$BIN_NAME" | tr -d ' ')"
printf '%s\n' "=============================="
printf '%s\n' "  ${C_BOLD}Build Successful${C_OFF}"
printf '%s\n' "=============================="
info "Platform    : $PLATFORM_TITLE"
info "Profile     : $PROFILE"
info "Version     : $VERSION"
info "Build Time  : $BUILD_TIME"
info "Binary Size : $(human_size "$BIN_BYTES")"
if [ "$ZIP_BYTES" -gt 0 ]; then
  info "Package Size: $(human_size "$ZIP_BYTES")"
fi
info "Output      : $RELEASE_DIR/"
printf '%s\n' "=============================="
printf '%s\n' ""
printf '%s\n' "${C_GREEN}✓ Copy the folder or the archive to any ${PLATFORM_TITLE} machine and run ${C_BOLD}${BIN_NAME}${C_OFF}${C_GREEN}. No install required.${C_OFF}"
