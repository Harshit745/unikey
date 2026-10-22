# ─────────────────────────────────────────────────────────────────────────────
# UniKey — run the full test suite (Windows / PowerShell)
#
#   .\scripts\test.ps1 [-Release]
#
#   (default) runs `cargo test --workspace`.
#   -Release  runs the suite against the release profile instead.
# ─────────────────────────────────────────────────────────────────────────────
[CmdletBinding()]
param([switch]$Release)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

if (-not (Get-Command "cargo" -ErrorAction SilentlyContinue)) {
    Write-Host "[error] cargo is required to run tests. Install Rust from https://rustup.rs" -ForegroundColor Red
    exit 1
}

$Start = Get-Date
Write-Host "[test] Running workspace tests$(if ($Release) { ' (release profile)' })…"
if ($Release) {
    & cargo test --workspace --release
} else {
    & cargo test --workspace
}
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$Elapsed = "{0}s" -f [math]::Round(((Get-Date) - $Start).TotalSeconds)
Write-Host "[test] Done in $Elapsed."
