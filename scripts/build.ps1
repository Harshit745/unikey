# ─────────────────────────────────────────────────────────────────────────────
# UniKey — build & package (Windows / PowerShell)
#
#   .\scripts\build.ps1 [-Clean] [-UI] [-NoZip] [-Profile release]
#
# Switches:
#   -Clean    wipe target\release and the release\ folder before building
#   -UI       also build the Tauri desktop shell and bundle it (requires npm)
#   -NoZip    build the release folder but skip the .zip archive
#   -Profile  build profile (default: release)
#
# Produces:
#   release\UniKey\                      portable application folder
#   release\UniKey-v<ver>-<OS>.zip       distributable archive
#   release\UniKey-Setup.exe             NSIS installer (with -UI)
#
# Fails gracefully with a readable message when cargo, git, assets, or the
# config are missing, when the build fails, or on permission problems.
# ─────────────────────────────────────────────────────────────────────────────
[CmdletBinding()]
param(
    [switch]$Clean,
    [switch]$UI,
    [switch]$NoZip,
    [string]$Profile = "release"
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Platform = "windows"   # PowerShell build script targets Windows only

. (Join-Path $PSScriptRoot "build\common.ps1")

if (-not (Test-Command "cargo")) {
    Write-Fail "cargo is required to build UniKey. Install Rust from https://rustup.rs (or add it to PATH)."
}

# ─── Metadata ───────────────────────────────────────────────────────────────
$Version = Get-WorkspaceVersion -RepoRoot $RepoRoot
$PlatformTitle = Get-PlatformTitle -Platform $Platform
$Commit = Get-GitCommit -RepoRoot $RepoRoot
$BuildDate = Get-BuildDate

$ReleaseRoot = Join-Path $RepoRoot "release"
$ReleaseDir  = Join-Path $ReleaseRoot "UniKey"
$ZipName     = "UniKey-v$Version-$PlatformTitle.zip"
$ZipPath     = Join-Path $ReleaseRoot $ZipName

$BinName = "daemon.exe"
$StartTime = Get-Date

# ─── Banner ─────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "=============================="
Write-Host "  UniKey Build"
Write-Host "=============================="
Write-Info "Platform : $PlatformTitle"
Write-Info "Profile  : $Profile"
Write-Info "Version  : $Version"

# ─── Clean (optional) ───────────────────────────────────────────────────────
if ($Clean) {
    Write-Info "Cleaning previous build artifacts…"
    & cargo clean --manifest-path (Join-Path $RepoRoot "Cargo.toml") -p daemon 2>$null
    if ($LASTEXITCODE -ne 0 -and $LASTEXITCODE -ne $null) {
        Write-Warn "cargo clean failed (continuing)."
    }
}
if (Test-Path $ReleaseRoot) { Remove-Item -Recurse -Force $ReleaseRoot }
New-Item -ItemType Directory -Path $ReleaseRoot -Force | Out-Null

# ─── Compile ────────────────────────────────────────────────────────────────
Write-Info "Compiling daemon ($Profile)…"
& cargo build --release --manifest-path (Join-Path $RepoRoot "Cargo.toml") -p daemon
if ($LASTEXITCODE -ne 0) {
    Write-Fail "Compilation failed. Re-run with verbose output, or consult docs\TROUBLESHOOTING.md."
}

# ─── Assemble release folder ────────────────────────────────────────────────
Write-Info "Assembling release folder…"
foreach ($Sub in @("config", "logs", "assets")) {
    New-Item -ItemType Directory -Path (Join-Path $ReleaseDir $Sub) -Force | Out-Null
}

# Portable marker — makes the daemon keep ALL state inside this folder.
Set-Content -Path (Join-Path $ReleaseDir "unikey.portable") -Value `
    "UniKey portable release. Removing this file switches the daemon to per-user config."

# 1. Binaries
$BinSrc = Join-Path $RepoRoot "target\release\$BinName"
if (-not (Test-Path $BinSrc)) { Write-Fail "Expected built binary at target\release\$BinName — was the build clean?" }
Copy-Item $BinSrc (Join-Path $ReleaseDir $BinName)
$BinBytes = (Get-Item (Join-Path $ReleaseDir $BinName)).Length
Write-Info "  - binary: $BinName ($(Format-Size $BinBytes) bytes)"

# Optional Tauri desktop shell (only with -UI). Stages the freshly built
# daemon as unikey-daemon.bin so the shell embeds it, then runs tauri build.
if ($UI) {
    if (Test-Command "npm") {
        Write-Info "Building Tauri desktop shell (this can take several minutes)…"
        Copy-Item -Force $BinSrc (Join-Path $RepoRoot "apps\desktop\src-tauri\unikey-daemon.bin")
        Push-Location (Join-Path $RepoRoot "apps\desktop")
        try {
            & npm ci
            if ($LASTEXITCODE -eq 0) { & npx tauri build --bundles nsis }
            $bundleDir = Join-Path $RepoRoot "apps\desktop\src-tauri\target\release\bundle"
            if ($LASTEXITCODE -eq 0 -and (Test-Path $bundleDir)) {
                Copy-Item -Recurse -Force $bundleDir (Join-Path $ReleaseDir "UI")
                $Installer = Get-ChildItem -Path $bundleDir -Recurse -Filter "*-setup.exe" | Select-Object -First 1
                if (-not $Installer) { Write-Fail "Tauri succeeded but no NSIS setup executable was found." }
                Copy-Item -Force $Installer.FullName (Join-Path $ReleaseRoot "UniKey-Setup.exe")
                Write-Info "  - UI bundle copied"
                Write-Info "  - installer: release\UniKey-Setup.exe"
            } else {
                Write-Warn "UI build failed; continuing without the UI bundle."
            }
        } finally { Pop-Location }
    } else {
        Write-Warn "-UI requested but npm is not installed; continuing without the UI."
    }
}

# 2. Assets (whole directory — future assets ship automatically)
if (Test-Path (Join-Path $RepoRoot "assets")) {
    Copy-Item -Recurse -Force (Join-Path $RepoRoot "assets") (Join-Path $ReleaseDir "assets")
    Write-Info "  - assets copied"
} else {
    Write-Warn "assets\ directory is missing at the repository root; shipping without assets."
}

# 3. Configuration (default template; the daemon recreates it if removed)
$ConfigSrc = Join-Path $RepoRoot "config\unikey.toml"
if (Test-Path $ConfigSrc) {
    Copy-Item $ConfigSrc (Join-Path $ReleaseDir "config\unikey.toml")
    Write-Info "  - config\unikey.toml copied"
} else {
    Write-Warn "config\unikey.toml is missing; the daemon will generate defaults on first run."
}

# 4. README.txt (from template) + LICENSE
$Template = Join-Path $PSScriptRoot "templates\README.txt"
if (Test-Path $Template) {
    Expand-ReadmeTemplate -Template $Template -Destination (Join-Path $ReleaseDir "README.txt") -Values @{
        "__VERSION__"    = $Version
        "__PLATFORM__"   = $PlatformTitle
        "__COMMIT__"     = $Commit
        "__BUILD_DATE__" = $BuildDate
    }
    Write-Info "  - README.txt generated"
} else {
    Write-Warn "README.txt template missing; skipping."
}
if (Test-Path (Join-Path $RepoRoot "LICENSE")) {
    Copy-Item (Join-Path $RepoRoot "LICENSE") (Join-Path $ReleaseDir "LICENSE")
    Write-Info "  - LICENSE copied"
} else {
    Write-Warn "LICENSE file missing; skipping."
}

# 5. version.json
New-VersionJson -Path (Join-Path $ReleaseDir "version.json") -Version $Version -Commit $Commit -BuildDate $BuildDate -Platform $PlatformTitle
Write-Info "  - version.json generated ($Version / $Commit / $BuildDate)"

# ─── Package ────────────────────────────────────────────────────────────────
$ZipBytes = 0
if (-not $NoZip) {
    Write-Info "Packaging $ZipName…"
    # Remove any stale archive first.
    if (Test-Path $ZipPath) { Remove-Item -Force $ZipPath }
    Compress-Archive -Path $ReleaseDir -DestinationPath $ZipPath -CompressionLevel Optimal
    $ZipBytes = (Get-Item $ZipPath).Length
}

# ─── Summary ────────────────────────────────────────────────────────────────
$BuildTime = "{0}s" -f [math]::Round(((Get-Date) - $StartTime).TotalSeconds)
Write-Host ""
Write-Host "=============================="
Write-Host "  Build Successful"
Write-Host "=============================="
Write-Info "Platform    : $PlatformTitle"
Write-Info "Profile     : $Profile"
Write-Info "Version     : $Version"
Write-Info "Build Time  : $BuildTime"
Write-Info "Binary Size : $(Format-Size $BinBytes)"
if ($ZipBytes -gt 0) { Write-Info "Package Size: $(Format-Size $ZipBytes)" }
Write-Info "Output      : $ReleaseDir\"
Write-Host "=============================="
Write-Host ""
Write-Host "✓ Copy the folder or the archive to any $PlatformTitle machine and run $BinName. No install required." -ForegroundColor Green
