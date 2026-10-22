#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# UniKey — clean build artifacts
#
#   ./scripts/clean.sh [--all]
#
#   (default) removes target/release and the release/ folder.
#   --all     removes the entire target/ directory (full rebuild next time).
# ─────────────────────────────────────────────────────────────────────────────
set -Eeuo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

ALL=0
for arg in "$@"; do
  case "$arg" in
    --all) ALL=1 ;;
    *) printf '%s\n' "Unknown option: $arg" >&2; exit 2 ;;
  esac
done

printf '%s\n' "[clean] Removing release/ …"
rm -rf "$REPO_ROOT/release"

if [ "$ALL" -eq 1 ]; then
  printf '%s\n' "[clean] Removing target/ (full rebuild required) …"
  rm -rf "$REPO_ROOT/target"
else
  printf '%s\n' "[clean] Removing target/release/ …"
  rm -rf "$REPO_ROOT/target/release"
fi

printf '%s\n' "[clean] Done."
