# ─────────────────────────────────────────────────────────────────────────────
# UniKey — shared PowerShell helpers (dot-sourced, not run).
#
# Expects these variables to be set by the caller:
#   $RepoRoot   absolute path to the repository root
#
# Provides the functions used by build.ps1 / clean.ps1 / test.ps1.
# ─────────────────────────────────────────────────────────────────────────────

# ANSI-friendly output helpers.
function Write-Info  { param([string]$Message) Write-Host "[info]  $Message" -ForegroundColor Green }
function Write-Warn  { param([string]$Message) Write-Host "[warn]  $Message" -ForegroundColor Yellow }
function Write-Fail  { param([string]$Message) Write-Host "[error] $Message" -ForegroundColor Red; exit 1 }

function Test-Command {
    param([string]$Name)
    return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

# Read the workspace version (e.g. 0.1.0) from the root Cargo.toml.
function Get-WorkspaceVersion {
    param([string]$RepoRoot)
    $line = Select-String -Path (Join-Path $RepoRoot "Cargo.toml") -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if ($null -eq $line) { return "0.0.0" }
    return $line.Matches[0].Groups[1].Value
}

# Human-readable OS used in the package filename.
function Get-PlatformTitle {
    param([string]$Platform)
    switch ($Platform) {
        "windows" { return "Windows" }
        "osx"     { return "macOS" }
        "linux"   { return "Linux" }
        default   { return "Unknown" }
    }
}

# Short git commit hash, or "unknown".
function Get-GitCommit {
    param([string]$RepoRoot)
    if (-not (Test-Command "git")) { return "unknown" }
    $hash = & git -C $RepoRoot rev-parse --short=12 HEAD 2>$null
    if ($LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($hash)) { return $hash.Trim() }
    return "unknown"
}

# UTC build date, or "unknown".
function Get-BuildDate {
    return (Get-Date -Format "yyyy-MM-dd" -AsUTC)
}

# Human-readable byte size (e.g. "12.4 MB").
function Format-Size {
    param([long]$Bytes)
    if ($Bytes -ge 1048576) { return "{0:N1} MB" -f ($Bytes / 1048576) }
    return "{0:N1} KB" -f ($Bytes / 1024)
}

# Write a version.json file.
function New-VersionJson {
    param([string]$Path, [string]$Version, [string]$Commit, [string]$BuildDate, [string]$Platform)
    $json = @{
        version  = $Version
        build    = $BuildDate
        commit   = $Commit
        platform = $Platform
    } | ConvertTo-Json
    [System.IO.File]::WriteAllText($Path, $json, (New-Object System.Text.UTF8Encoding $false))
}

# Replace placeholders in the README.txt template.
function Expand-ReadmeTemplate {
    param([string]$Template, [string]$Destination, [hashtable]$Values)
    $text = [System.IO.File]::ReadAllText($Template)
    foreach ($key in $Values.Keys) { $text = $text.Replace($key, [string]$Values[$key]) }
    [System.IO.File]::WriteAllText($Destination, $text, (New-Object System.Text.UTF8Encoding $false))
}
