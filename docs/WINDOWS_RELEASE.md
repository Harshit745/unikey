# Windows desktop release

UniKey's supported Windows package is a 64-bit **NSIS setup executable** named
`UniKey-Setup.exe`. It is built on a genuine `windows-latest` GitHub Actions
runner from the same commit as the Rust daemon and React/Tauri shell.

## What the installer contains

- `UniKey.exe`: the Tauri v2 desktop application, compiled as a GUI process
  with no console window.
- The matching release `daemon.exe`, embedded in `UniKey.exe` and extracted on
  first launch. Users do not need Rust, Node.js, npm, Cargo, Git, or Visual
  Studio.
- The compiled React assets and the existing UniKey icon/metadata.
- A Start menu shortcut, an optional desktop shortcut on the installer's
  finish page, and `uninstall.exe` plus an Apps & Features entry.
- The WebView2 bootstrapper download path. Windows 10/11 normally already has
  WebView2; if it is absent, setup downloads Microsoft's official bootstrapper.

The installer is **current-user** by default and does not request elevation.
It installs under `%LOCALAPPDATA%\UniKey`. Downgrades are blocked; running a
newer installer over an older version performs the normal NSIS upgrade flow.

## Runtime lifecycle and storage

The desktop application owns one daemon child process. The UI starts it,
waits for `http://127.0.0.1:45895/health`, and retries with a bounded backoff
after a crash. Closing the main window hides UniKey to the tray. Choosing
**Quit** exits the app and stops only the child daemon it started. An already
running external daemon is never terminated by the desktop shell.

The embedded daemon is extracted to:

```text
%LOCALAPPDATA%\UniKey\runtime\unikey-daemon-<fingerprint>.exe
```

The content-addressed filename lets an upgrade stage a new executable without
trying to overwrite a process Windows still has open. Mutable identity, trust,
keys, configuration, logs, and received-file state remain under the daemon's
per-user data root:

```text
%APPDATA%\UniKey\
```

Normal uninstall removes the application and shortcuts. User state is not
silently removed; the interactive uninstaller presents Tauri's separate
**Delete app data** choice. Preserve that data for upgrades or reinstallations.

## Network and permissions

UniKey does not install a Windows service, driver, scheduled task, global
keyboard hook, or startup entry. It does not require administrator privileges
for installation or normal use.

| Purpose | Binding | Permission required |
| --- | --- | --- |
| Desktop UI API | TCP `127.0.0.1:45895` | None; localhost only, never add a firewall rule |
| Encrypted peer transport | TCP `0.0.0.0:45892` | Private-network inbound firewall access for cross-device use |
| Device discovery | UDP `0.0.0.0:45893` | Private-network inbound firewall access for cross-device use |

The installer deliberately does **not** request an administrator password to
add firewall rules. Windows may show its standard firewall prompt on first
network use. Allow UniKey on **Private networks only**. If policy suppresses
the prompt, an administrator may add the two narrow rules documented in
[WINDOWS_TESTING.md](WINDOWS_TESTING.md). Do not expose port 45895.

Clipboard and input features use ordinary user-session APIs. Windows input
forwarding has not yet been verified across physical devices and may be
limited by elevated target applications, secure desktop/UAC prompts, lock
screens, or endpoint-security policy. UniKey does not bypass those boundaries.

## Build locally on Windows

Prerequisites for a source build only:

- Windows 10 or 11, x64
- Rust stable with the `x86_64-pc-windows-msvc` target and MSVC build tools
- Node.js 22 and npm
- WebView2 runtime

From PowerShell at the repository root:

```powershell
.\scripts\build.ps1 -Clean -UI
```

The professional installer is copied to:

```text
release\UniKey-Setup.exe
```

The portable daemon archive is still produced separately. It is useful for
engineering diagnostics, but it is not the recommended end-user download.

## Build and verify in GitHub Actions

Run the **UniKey Desktop** workflow manually, or push a reviewed commit to
`main`. The Windows job:

1. installs locked npm dependencies;
2. type-checks and builds the React frontend;
3. runs the Rust workspace and Tauri host tests on MSVC;
4. builds the release daemon and embeds it in the shell;
5. builds only the NSIS target;
6. silently installs the resulting setup file on the Windows runner;
7. launches the installed `UniKey.exe`, waits for the real embedded daemon,
   checks graceful app/daemon exit, and runs the uninstaller;
8. uploads `UniKey-Setup.exe` and its SHA-256 file as the
   `UniKey-Windows-x64` workflow artifact.

The reproducible smoke command is:

```powershell
.\scripts\windows\installer-smoke.ps1 `
  -InstallerPath .\release-artifacts\UniKey-Setup.exe
```

Use it only on a clean VM because installing the same product over a personal
installation is a real upgrade. The script does not fake device communication.

## Code signing and publication

The current repository contains no signing certificate or signing secret.
Unsigned development installers may trigger Microsoft Defender SmartScreen;
that warning must not be bypassed or described as trusted. Before a public
release, configure a reputable Windows code-signing certificate in encrypted
CI secrets and timestamp both the application and installer.

Ordinary CI runs create expiring workflow artifacts. A `v*` tag is configured
to publish a GitHub Release, so do not push a release tag until the installer,
physical-device tests, and signing configuration have been approved.

## Verification boundary

Passing CI proves that the x64 installer builds on Windows, installs, launches,
starts the real packaged daemon, exits cleanly, and uninstalls on the hosted
runner. It does **not** prove cross-device discovery, pairing, file transfer,
clipboard, input forwarding, Wi-Fi recovery, sleep/wake recovery, ARM64
support, or enterprise-managed Windows compatibility. Complete
[WINDOWS_TESTING.md](WINDOWS_TESTING.md) on the actual Windows laptop before
making those claims.
