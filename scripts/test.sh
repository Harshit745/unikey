#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# UniKey — run the full test suite (designed to finish in under a minute)
#
#   ./scripts/test.sh [--release]
#
#   (default) runs `cargo test --workspace`.
#   --release runs the suite against the release profile instead.
# ─────────────────────────────────────────────────────────────────────────────
set -Eeuo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

command -v cargo >/dev/null 2>&1 || {
  printf '%s\n' "[error] cargo is required to run tests. Install Rust from https://rustup.rs" >&2
  exit 1
}

START="$(date +%s)"
PROFILE_FLAG=""
for arg in "$@"; do
  case "$arg" in
    --release) PROFILE_FLAG="--release" ;;
    *) printf '%s\n' "Unknown option: $arg" >&2; exit 2 ;;
  esac
done

printf '%s\n' "[test] Running workspace tests${PROFILE_FLAG:+ (release profile)}…"
# shellcheck disable=SC2086
cargo test --workspace $PROFILE_FLAG
ELAPSED="$(( $(date +%s) - START ))s"
printf '%s\n' "[test] Done in $ELAPSED."
