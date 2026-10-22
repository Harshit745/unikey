# Testing UniKey between macOS and Windows

This checklist verifies the current MVP across two physical computers. It does
not treat a successful build as proof of cross-platform interoperability.

## 1. Obtain matching builds

Use installers from the same tagged GitHub Release:

- `UniKey-macOS-universal.dmg`
- `UniKey-Windows-x64-Setup.exe`

Until a tagged release exists, build on each operating system from the same
commit:

```bash
# macOS
./scripts/build.sh --clean --ui
```

```powershell
# Windows PowerShell
.\scripts\build.ps1 -Clean -UI
```

Do not copy a macOS daemon binary into the Windows application. The desktop
build embeds the platform-specific release daemon automatically.

## 2. Prepare the network

1. Put both computers on the same trusted private Wi-Fi or Ethernet LAN.
2. Disable VPNs temporarily. Do not use a guest network; many guest networks
   block device-to-device traffic.
3. On Windows, ensure the network profile is **Private**:

   ```powershell
   Get-NetConnectionProfile
   ```

4. When Windows Defender Firewall prompts, allow UniKey on **Private
   networks** only. If no prompt appears, an administrator can add narrowly
   scoped inbound rules:

   ```powershell
   New-NetFirewallRule -DisplayName "UniKey transport" -Direction Inbound -Protocol TCP -LocalPort 45892 -Action Allow -Profile Private
   New-NetFirewallRule -DisplayName "UniKey discovery" -Direction Inbound -Protocol UDP -LocalPort 45893 -Action Allow -Profile Private
   ```

Do not expose TCP 45895 in the firewall. It is the UI API and must remain bound
to localhost.

## 3. Confirm each daemon independently

Launch UniKey on both computers. Each client should display **Daemon online**.

On macOS:

```bash
curl http://127.0.0.1:45895/health
```

On Windows PowerShell:

```powershell
Invoke-RestMethod http://127.0.0.1:45895/health
```

The expected status is `ok`. If either check fails, inspect the Logs page before
testing discovery.

## 4. Verify discovery and pairing

1. Open **Devices** on both computers.
2. Wait up to ten seconds for each machine to show the other as nearby.
3. On one computer, open **Pairing** and generate a six-digit code.
4. On the other, select the discovered peer, enter the code, and submit.
5. Confirm both clients record a successful pairing and an active encrypted
   session.

Only test pairing on a trusted LAN. Pairing-attempt throttling is still an open
MVP hardening item.

## 5. Verify a real file transfer

1. Create a small, non-sensitive text file on the sender.
2. Open **Files**, select the paired device, enter the full local path, and send.
3. Confirm both clients show the transfer as `Verified`.
4. Compare hashes rather than relying only on the UI.

macOS:

```bash
shasum -a 256 /path/to/original-file
shasum -a 256 /path/to/received-file
```

Windows PowerShell:

```powershell
Get-FileHash -Algorithm SHA256 C:\path\to\original-file
Get-FileHash -Algorithm SHA256 C:\path\to\received-file
```

The two hashes must match.

## 6. Recovery checks

Repeat these separately and record the result:

1. Close and reopen UniKey on Windows; persisted trust should remain.
2. Turn Windows Wi-Fi off for 20 seconds, restore it, and wait for reconnection.
3. Put the Windows laptop to sleep, wake it, and verify rediscovery.
4. Transfer files in both directions.
5. Try a wrong pairing code and confirm the request is rejected without adding
   a trusted device.
6. Remove the trusted device, then confirm protected transfers fail until the
   devices are paired again.

## 7. Evidence to retain

For an honest compatibility claim, save:

- the Git commit or release tag tested;
- Windows edition/build and macOS version;
- whether the connection used Wi-Fi or Ethernet;
- screenshots of both Devices pages after pairing;
- screenshots of both transfer rows showing `Verified`;
- matching SHA-256 values;
- relevant daemon logs for any failed step.

Claim Windows compatibility only after this physical-device checklist passes.
