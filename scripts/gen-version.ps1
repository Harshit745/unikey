# ─────────────────────────────────────────────────────────────────────────────
# UniKey — generate version.json (Windows / PowerShell)
#
#   .\scripts\gen-version.ps1 [-Out path]
#
# Writes JSON metadata to stdout when -Out is omitted, otherwise to the path
# (creating parent directories). Used by the build pipeline and CI.
# ─────────────────────────────────────────────────────────────────────────────
[CmdletBinding()]
param([string]$Out = "")

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
. (Join-Path $PSScriptRoot "build\common.ps1")

$Version = Get-WorkspaceVersion -RepoRoot $RepoRoot
$Commit  = Get-GitCommit -RepoRoot $RepoRoot
$Date    = Get-BuildDate
$Platform = "Windows"

if ([string]::IsNullOrWhiteSpace($Out)) {
    $json = @{ version = $Version; build = $Date; commit = $Commit; platform = $Platform } | ConvertTo-Json
    Write-Output $json
} else {
    New-VersionJson -Path $Out -Version $Version -Commit $Commit -BuildDate $Date -Platform $Platform
    Write-Host "version.json -> $Out"
}
