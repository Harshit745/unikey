#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# UniKey — generate version.json for a release
#
#   ./scripts/gen-version.sh [output-path]
#
# Writes JSON metadata to stdout when no path is given, otherwise to the path
# (creating parent directories). Used by the build pipeline and CI.
# ─────────────────────────────────────────────────────────────────────────────
set -Eeuo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$REPO_ROOT/scripts/build/common.sh"

OUT="${1:-/dev/stdout}"
# Allow an explicit override (CI cross-compilation), otherwise auto-detect.
PLATFORM="${PLATFORM:-}"
if [ -z "$PLATFORM" ]; then
  case "$(uname -s 2>/dev/null || true)" in
    Darwin) PLATFORM="osx" ;;
    Linux)  PLATFORM="linux" ;;
    MINGW*|MSYS*|CYGWIN*) PLATFORM="windows" ;;
    *) PLATFORM="unknown" ;;
  esac
fi
case "$PLATFORM" in
  windows) PLATFORM_TITLE="Windows" ;;
  osx|macos) PLATFORM_TITLE="macOS" ;;
  linux) PLATFORM_TITLE="Linux" ;;
  *) PLATFORM_TITLE="Unknown" ;;
esac

VERSION="$(version_from_cargo)"
COMMIT="$(git_commit)"
DATE="$(build_date)"

if [ "$OUT" != "/dev/stdout" ]; then
  mkdir -p "$(dirname "$OUT")"
fi
gen_version_json "$OUT" "$VERSION" "$COMMIT" "$DATE" "$PLATFORM_TITLE"
printf '%s\n' "version.json → $OUT"
