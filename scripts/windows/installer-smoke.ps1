# Runs the packaged Windows application through install, real daemon startup,
# API checks, graceful app/daemon exit, and uninstall. Use a clean Windows VM:
# installing the same product over a personal UniKey installation is an
# upgrade and is intentionally not supported by this smoke test.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$InstallerPath,
    [string]$InstallDir = (Join-Path $env:RUNNER_TEMP "UniKey-Installer-Smoke"),
    [string]$ExpectedCommit,
    [string]$ExpectedDaemonPath,
    [string]$ExpectedAppPath
)

$ErrorActionPreference = "Stop"
$InstallerPath = (Resolve-Path $InstallerPath).Path

if (Test-Path $InstallDir) {
    throw "Smoke install directory already exists: $InstallDir"
}
if (Get-NetTCPConnection -LocalPort 45895 -State Listen -ErrorAction SilentlyContinue) {
    throw "Port 45895 is already occupied; refusing to validate an unrelated daemon"
}
if ($ExpectedDaemonPath) { $ExpectedDaemonPath = (Resolve-Path $ExpectedDaemonPath).Path }
if ($ExpectedAppPath) { $ExpectedAppPath = (Resolve-Path $ExpectedAppPath).Path }

Write-Host "Installing $InstallerPath into $InstallDir"
$install = Start-Process -FilePath $InstallerPath -ArgumentList "/S", "/D=$InstallDir" -Wait -PassThru
if ($install.ExitCode -ne 0) { throw "Installer exited with code $($install.ExitCode)" }

$AppPath = Join-Path $InstallDir "UniKey.exe"
$UninstallerPath = Join-Path $InstallDir "uninstall.exe"
if (-not (Test-Path $AppPath)) { throw "Installed app not found: $AppPath" }
if (-not (Test-Path $UninstallerPath)) { throw "Uninstaller not found: $UninstallerPath" }
if ($ExpectedAppPath -and
    (Get-FileHash $AppPath -Algorithm SHA256).Hash -ne (Get-FileHash $ExpectedAppPath -Algorithm SHA256).Hash) {
    throw "Installed app does not match the newly built shell (stale installer)"
}

$StartMenuShortcut = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\UniKey\UniKey.lnk"
if (-not (Test-Path $StartMenuShortcut)) { throw "Start menu shortcut not found: $StartMenuShortcut" }

Write-Host "Launching installed application and waiting for its real daemon"
$app = Start-Process -FilePath $AppPath -ArgumentList "--installer-smoke-test" -PassThru
$healthDeadline = (Get-Date).AddSeconds(30)
do {
    if ($app.HasExited) { throw "Installed application exited before its daemon became healthy" }
    try {
        $health = Invoke-RestMethod -Uri "http://127.0.0.1:45895/health" -TimeoutSec 1
        if ($health.status -eq "ok") { break }
    } catch {
        Start-Sleep -Milliseconds 250
    }
} while ((Get-Date) -lt $healthDeadline)
if ($health.status -ne "ok") { throw "Packaged daemon did not become healthy within 30 seconds" }

$version = Invoke-RestMethod -Uri "http://127.0.0.1:45895/version" -TimeoutSec 2
$services = Invoke-RestMethod -Uri "http://127.0.0.1:45895/services" -TimeoutSec 2
if (-not $version.version) { throw "Packaged daemon /version response was incomplete" }
if ($null -eq $services.services) { throw "Packaged daemon /services response was incomplete" }
Write-Host "Packaged daemon version: $($version | ConvertTo-Json -Compress)"
if ($ExpectedCommit -and $version.commit -ne $ExpectedCommit) {
    throw "Packaged daemon commit $($version.commit) does not match CI source $ExpectedCommit"
}

$cursor = Invoke-RestMethod -Uri "http://127.0.0.1:45895/api/v1/input/cursor" -TimeoutSec 2
if ($cursor.success -ne $true -or $cursor.data.supported -ne $true -or
    $cursor.data.receiving_enabled -ne $false -or $cursor.data.phase -ne "local") {
    throw "Packaged daemon cursor API is missing or has unsafe startup state"
}
Write-Host "Packaged cursor API: supported, receiving off, local ownership"

$listeners = @(Get-NetTCPConnection -LocalPort 45895 -State Listen)
$ownerIds = @($listeners.OwningProcess | Sort-Object -Unique)
if ($ownerIds.Count -ne 1) { throw "Expected one packaged daemon API owner" }
$daemonProcess = Get-CimInstance Win32_Process -Filter "ProcessId = $($ownerIds[0])"
if ($daemonProcess.ParentProcessId -ne $app.Id) { throw "API daemon is not owned by the installed app" }
if ($ExpectedDaemonPath -and
    (Get-FileHash $daemonProcess.ExecutablePath -Algorithm SHA256).Hash -ne
    (Get-FileHash $ExpectedDaemonPath -Algorithm SHA256).Hash) {
    throw "Running embedded daemon does not match the newly built daemon"
}
Write-Host "Verified daemon owner and payload: $($daemonProcess.ExecutablePath)"

$app | Wait-Process -Timeout 15
$app.Refresh()
if ($app.ExitCode -ne 0) { throw "Installed application smoke mode exited with code $($app.ExitCode)" }

# Smoke mode exits only after /health succeeds. Verify that the graceful app
# exit also released the localhost API port by stopping its owned child.
$deadline = (Get-Date).AddSeconds(10)
do {
    try {
        Invoke-RestMethod -Uri "http://127.0.0.1:45895/health" -TimeoutSec 1 | Out-Null
        Start-Sleep -Milliseconds 250
    } catch {
        $daemonStopped = $true
        break
    }
} while ((Get-Date) -lt $deadline)
if (-not $daemonStopped) { throw "Daemon API remained reachable after graceful app exit" }

$RuntimeDir = Join-Path $env:LOCALAPPDATA "UniKey\runtime"
$EmbeddedDaemon = Get-ChildItem -Path $RuntimeDir -Filter "unikey-daemon-*.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $EmbeddedDaemon) { throw "No extracted embedded daemon found in $RuntimeDir" }

Write-Host "Uninstalling UniKey"
$uninstall = Start-Process -FilePath $UninstallerPath -ArgumentList "/S" -Wait -PassThru
if ($uninstall.ExitCode -ne 0) { throw "Uninstaller exited with code $($uninstall.ExitCode)" }
$uninstallDeadline = (Get-Date).AddSeconds(10)
while ((Test-Path $AppPath) -and (Get-Date) -lt $uninstallDeadline) {
    Start-Sleep -Milliseconds 250
}
if (Test-Path $AppPath) { throw "Application remained after uninstall: $AppPath" }

Write-Host "Installer smoke test passed: install, current app/daemon payload, commit, cursor API, graceful exit, and uninstall"
