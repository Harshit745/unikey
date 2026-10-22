# ─────────────────────────────────────────────────────────────────────────────
# UniKey — clean build artifacts (Windows / PowerShell)
#
#   .\scripts\clean.ps1 [-All]
#
#   (default) removes target\release and the release\ folder.
#   -All      removes the entire target\ directory (full rebuild next time).
# ─────────────────────────────────────────────────────────────────────────────
[CmdletBinding()]
param([switch]$All)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

if (Test-Path (Join-Path $RepoRoot "release")) {
    Remove-Item -Recurse -Force (Join-Path $RepoRoot "release")
}

$TargetDir = Join-Path $RepoRoot "target"
if (Test-Path $TargetDir) {
    if ($All) {
        Write-Host "[clean] Removing target\ (full rebuild required) …"
        Remove-Item -Recurse -Force $TargetDir
    } else {
        $ReleaseTarget = Join-Path $TargetDir "release"
        if (Test-Path $ReleaseTarget) {
            Write-Host "[clean] Removing target\release\ …"
            Remove-Item -Recurse -Force $ReleaseTarget
        }
    }
}

Write-Host "[clean] Done."
