#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# UniKey — shared bash helpers for the build/release scripts (sourced, not run).
#
# Expects these variables to be set by the caller:
#   REPO_ROOT   absolute path to the repository root
#   PLATFORM    lower-case OS: "macos" | "linux" | "windows"
#   PROFILE     "release" (debug builds are not directly supported)
#
# Provides:
#   repo_root, fail, warn, info, banner,
#   require_cmd, platform_from_uname, version_from_cargo, git_commit,
#   build_date, human_size, make_release_dir, gen_version_json,
#   fill_readme_template
# ─────────────────────────────────────────────────────────────────────────────
set -Eeuo pipefail

# ─── Colours (disabled when piped / non-tty) ────────────────────────────────
if [[ -t 1 ]]; then
    readonly C_RED=$'\033[31m' C_GREEN=$'\033[32m' C_YELLOW=$'\033[33m'
    readonly C_BOLD=$'\033[1m' C_DIM=$'\033[2m' C_OFF=$'\033[0m'
else
    readonly C_RED= C_GREEN= C_YELLOW= C_BOLD= C_DIM= C_OFF=
fi

# ─── Logging ────────────────────────────────────────────────────────────────
info()  { printf '%s\n' "${C_GREEN}[info]${C_OFF} ${C_DIM}${1}${C_OFF}"; }
warn()  { printf '%s\n' "${C_YELLOW}[warn]${C_OFF} ${1}" >&2; }
fail()  { printf '%s\n' "${C_RED}[error]${C_OFF} ${1}" >&2; exit 1; }

repo_root() { printf '%s' "${REPO_ROOT:?caller must set REPO_ROOT}"; }

# ─── Command availability ───────────────────────────────────────────────────
require_cmd() {
  local name="$1"; shift
  if ! command -v "$name" >/dev/null 2>&1; then
    fail "'$name' is required but was not found on PATH. ${1:-}"
  fi
}

# ─── Version / platform discovery ───────────────────────────────────────────
# Read the workspace version (e.g. 0.1.0) from the root Cargo.toml.
version_from_cargo() {
  local line version
  line="$(grep -m1 '^version' "$REPO_ROOT/Cargo.toml" || true)"
  version="${line#version = }"
  version="${version%\"}"
  version="${version#\"}"
  [ -n "$version" ] || version="0.0.0"
  printf '%s' "$version"
}

# Human-readable OS used in the package filename.
platform_title() {
  case "$PLATFORM" in
    windows) printf 'Windows' ;;
    osx)     printf 'macOS' ;;
    linux)   printf 'Linux' ;;
    *)       printf 'Unknown' ;;
  esac
}

# Short git commit hash, or "unknown".
git_commit() {
  if command -v git >/dev/null 2>&1 && git -C "$REPO_ROOT" rev-parse --short=12 HEAD >/dev/null 2>&1; then
    git -C "$REPO_ROOT" rev-parse --short=12 HEAD
  else
    printf 'unknown'
  fi
}

# UTC build date, or "unknown".
build_date() {
  if command -v date >/dev/null 2>&1; then
    date -u +%Y-%m-%d 2>/dev/null || date -u +'%Y-%m-%d'
  else
    printf 'unknown'
  fi
}

# Human-readable byte size (e.g. "12.4 MB").
human_size() {
  local bytes="$1"
  if [ "$bytes" -lt 1048576 ]; then
    awk "BEGIN{printf \"%.1f KB\", $bytes/1024}"
  else
    awk "BEGIN{printf \"%.1f MB\", $bytes/1048576}"
  fi
}

# ─── Release folder construction ────────────────────────────────────────────
mkdir_p() { mkdir -p "$1"; }

gen_version_json() {
  local out="$1" version="$2" commit="$3" date="$4" platform="$5"
  cat > "$out" <<EOF
{
  "version": "$version",
  "build": "$date",
  "commit": "$commit",
  "platform": "$platform"
}
EOF
}

# Render the README.txt template, substituting placeholders.
fill_readme_template() {
  local src="$1" dst="$2"
  sed -e "s/__VERSION__/${VERSION}/g" \
      -e "s/__PLATFORM__/${PLATFORM_TITLE:-$PLATFORM}/g" \
      -e "s/__COMMIT__/${COMMIT_SHORT}/g" \
      -e "s/__BUILD_DATE__/${BUILD_DATE}/g" \
      "$src" > "$dst"
}