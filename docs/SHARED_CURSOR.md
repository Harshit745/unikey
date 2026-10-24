# Shared cursor (experimental)

UniKey can hand a mouse between a paired Mac and Windows computer through its
existing encrypted TCP session. Both computers must run the cursor-capable
build; an older peer will time out before any local input is suppressed.

## Setup

1. Keep the computers on the same LAN, discover them, and complete pairing.
2. On the receiving computer, open **Input → Allow paired cursor**.
3. On the computer whose mouse you will use, open **Input**, select the paired
   computer and its physical position (**Left** or **Right**), then **Start sharing**.
4. Wait for **Ready at the shared edge**. Move the mouse into that edge without
   holding a button. The receiver confirms ownership before local capture starts.
5. Move and click on the remote display; scroll or drag within a remote app.
6. Move back across the shared edge to return. **Escape on the source keyboard**
   stops sharing immediately. **Stop sharing** and daemon shutdown also release
   ownership. Loss of authenticated heartbeats returns control within three seconds.

Receiving is off at every daemon startup and is never enabled by a network packet.
Only the selected trusted identity can accept ownership. Random or stale lease
IDs and out-of-range coordinates are rejected. Held mouse buttons are released
when ownership ends or the connection expires. Local drags do not cross devices.

## macOS permission and packaging

Native capture requires **Privacy & Security → Accessibility** (called **Device
Control and Data Access** on newer macOS versions). Enable UniKey there and retry.
The packaged app runs a separate, content-addressed daemon. If capture still
reports that permission is missing, also add the exact running
`unikey-daemon-<hash>` executable from
`~/Library/Application Support/UniKey/runtime/` to that permission panel.
Verify its path against the running UniKey helper before granting access; do not
grant access to unrelated executables. Quit and reopen UniKey after granting it.
A new helper binary after an update may require its own permission grant.
If the switches are on but native capture still reports permission denied after
an ad-hoc-signed update, toggling alone may retain the old code requirement.
Remove and re-add the current `/Applications/UniKey.app` and the exact current
helper through that same macOS permission panel, then fully quit and reopen
UniKey. Do not edit the OS permission database or bypass the native trust check.
An unavailable OS backend is reported as unavailable, never simulated as working.
Only Escape is inspected from the keyboard; keyboard contents are not forwarded.

The Mac bundle is ad-hoc signed with the stable `dev.unikey.desktop` identifier
and includes a Local Network usage description. This avoids the incomplete
linker-only signature that caused discovery permission attribution trouble in
the earlier local build. Distribution signing and notarization remain separate.

## Troubleshooting an old daemon after a Windows update

The desktop launcher reuses any already-healthy daemon on port `45895`. A daemon
left behind by an earlier app instance can therefore survive an app reinstall.
The earlier Input page's update message was a generic API-error message, not a
verified version diagnosis. The updated page distinguishes a missing cursor API
(404) from other connection errors. In Command Prompt, inspect both endpoints:

```cmd
curl.exe http://127.0.0.1:45895/version
curl.exe -i http://127.0.0.1:45895/api/v1/input/cursor
```

An October 22 build with commit `f9b523653713` predates shared cursor support and
returns 404 for that route. Fully quit UniKey from the system tray; the window's
X only hides it. With no transfer or sharing active, restart Windows to clear an
orphaned daemon, then open the newly installed UniKey from Start. Do not delete
the pairing store or runtime folder. If the old version persists after a full
Windows restart, inspect the installed app path and installer checksum before
reinstalling again.

Windows packaging now selects only the explicit MSVC target's installer, never
the first cached installer under `target/`. Its smoke test checks the running
daemon against the freshly built daemon by SHA-256, verifies
the source commit, real parent process, cursor endpoint and safe startup state,
and refuses to run if another daemon already owns the API port.
The GUI shell is patched by Tauri with an NSIS bundle-type marker, so its installed
hash is not compared to the restored, unbundled shell executable.

## Current boundaries and verification

- Native macOS Quartz event tap and Windows low-level mouse hook implementations.
- One main display per computer, left/right layout; additional monitors and
  vertical layouts are not supported yet.
- Mouse only. Keyboard handoff and continuous file dragging into remote apps
  are separate work. Windows elevated apps and the secure desktop are outside scope.
- Geometry is negotiated in each OS's desktop coordinate system; different
  display heights map the crossing position proportionally.
- Tests cover ownership acknowledgement, reverse crossing, local drag exclusion,
  emergency release, timeout, explicit receiver opt-in, peer/lease validation,
  screen bounds, and held-button release. These protocol tests use a recording
  injector and do not constitute physical mouse testing.
- Run `cargo test --workspace`, `cargo clippy -p input --all-targets -- -D warnings`,
  `cargo check -p input --target x86_64-pc-windows-gnu`, and
  `cd apps/desktop && npm run build`.
- Physical Mac ↔ Dell cursor testing must be done with the updated build on both
  computers and the Mac permission granted. Packaging CI alone cannot verify feel,
  local cursor suppression, or cross-device physical movement.

## Low-latency path

- Both ends of every authenticated TCP connection use `TCP_NODELAY`, including
  accepted sockets. Encryption, replay protection and pairing checks are unchanged.
- The native-to-service queue replaces only adjacent absolute movement for the
  same device and ownership lease. During a burst the newest position replaces
  stale positions instead of replaying an ever-older trail. Button down/up,
  scrolling, peer/lease changes and ownership messages are ordering barriers;
  they are never coalesced. Discrete-event overflow still restores local control.
- Transport output and daemon input also collapse already-buffered adjacent
  positions for the same peer/lease before encryption and before injection,
  respectively. No extra timing window or smoothing filter is added. This
  prevents a backlog from replaying old cursor positions while aiming at small
  targets. Receiver pixel conversion rounds rather than truncating positions.
- The Windows hook waits on input messages rather than sleeping between polls.
  Mouse input wakes it immediately; a short timeout services Escape and recovery.
- No predictive movement or interpolation is used: these can add latency or
  produce motion the user did not make. Network quality, display refresh rate
  and OS injection still affect the physical result. Bulk file traffic shares
  the same TCP connection and can still cause head-of-line blocking; a dedicated
  authenticated realtime channel is future work, not a claimed implementation.
- Reproduce the host-dependent encrypted loopback timing check (not a two-device
  or mouse-to-photon benchmark):
  `cargo test -p transport --test secure_session cursor_stream_latency -- --ignored --nocapture`.
  Unit tests additionally verify a 10,000-move burst collapses to the latest
  position without losing click/scroll/release ordering.

### Precision update checkpoint (2026-10-24)

Source build `3439c82` additionally coalesces already-buffered absolute cursor
positions at the transport send and daemon receive queues, without crossing
peer, lease, button, scroll or ownership boundaries. Native injection rounds
absolute coordinates rather than truncating, and Quartz relative motion reads
the documented integer delta fields. No artificial smoothing or pointer gain
was added.

Local workspace tests, four frontend tests, eleven desktop-shell tests, targeted
input clippy, Windows cross-compilation and Mac packaging passed. A real encrypted
TCP regression test preserves the final motion position before each button event.
[Desktop CI 40085960363](https://github.com/Harshit745/unikey/actions/runs/40085960363)
passed Windows tests and NSIS install/launch/current-payload/shutdown/uninstall
verification, macOS packaging and Linux integration. The [Windows artifact](https://github.com/Harshit745/unikey/actions/runs/40085960363/artifacts/12556694885)
has independently checked installer SHA-256
`dadb8778b50a9269c24541bb09ad0c471a2348abfbaee8fb28500fdab9d9805c`.

The installed Mac's approved app/helper grants were refreshed and native capture
started successfully. Receiving was turned off afterward. Physical precision
and smoothness remain subject to a user retest with this build on both computers;
neither successful compilation nor these protocol tests establish mouse-to-photon
latency or Apple-equivalent feel.

### Earlier low-latency checkpoint (2026-10-24)

The low-latency update is source commit `a424a96`. [Desktop CI run
40083511083](https://github.com/Harshit745/unikey/actions/runs/40083511083)
passed all jobs, including the real Windows packaged daemon's full source
commit, SHA-256 payload comparison, safe cursor API state and install/exit/uninstall.
The [Windows artifact](https://github.com/Harshit745/unikey/actions/runs/40083511083/artifacts/12556685802)
contains `UniKey-Setup.exe` with independently checked SHA-256
`87ff9c0d7f95a45e4afad982af1e1201272ecbd534a454f4d80f2d86a7695b9f`.
Local workspace regression tests, 18 input tests, targeted input clippy, Windows
cross-compilation and the Mac release build passed. Host-only encrypted loopback
timing was p50 109 µs / p95 179 µs in one 200-packet run; this is not a physical
two-device or mouse-to-photon latency claim. Ad-hoc-signed app/helper updates can
require refreshing their individual macOS Accessibility grants before capture
works again. Physical smoothness of this update still requires a user retest on
both updated computers.

Earlier feature checkpoint:

- Source commit `3866e63`: local Rust workspace tests, targeted input clippy,
  Windows cross-compilation, frontend type-check/build, and desktop-shell tests
  passed. Strict daemon clippy still reports an existing `collapsible_if` warning
  in the pairing API handler; it was not reformatted as part of this feature.
- The installed Mac app successfully started a real Quartz event tap with the
  approved helper permission and reported a `1710 × 1112` main display. Receiving
  was turned off again after that check. This is not a physical two-device test.
- [Desktop CI run 40058169108](https://github.com/Harshit745/unikey/actions/runs/40058169108)
  passed macOS packaging, Linux daemon integration, and Windows tests/packaging.
  Windows installed the NSIS package, launched the packaged app and embedded
  daemon, verified API readiness and graceful exit, then uninstalled it.
- [Windows artifact: UniKey-Windows-x64](https://github.com/Harshit745/unikey/actions/runs/40058169108/artifacts/12546676146)
  contains `UniKey-Setup.exe` and its checksum file. The downloaded EXE checksum
  was independently verified:
  `77c18f4b1d67fd29ed3f8b2da8acf491f35611bba3f111f63865a6a45b7ca5dc`.
  CI artifacts expire and may require GitHub sign-in; this is not a published release.
