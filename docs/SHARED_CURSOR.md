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
An unavailable OS backend is reported as unavailable, never simulated as working.
Only Escape is inspected from the keyboard; keyboard contents are not forwarded.

The Mac bundle is ad-hoc signed with the stable `dev.unikey.desktop` identifier
and includes a Local Network usage description. This avoids the incomplete
linker-only signature that caused discovery permission attribution trouble in
the earlier local build. Distribution signing and notarization remain separate.

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
