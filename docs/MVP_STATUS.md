# UniKey MVP audit and priority checklist

Audit date: 2026-10-21

## Repository state inspected

- Active local checkout: `/Users/harshitsharma/Development/unikey`
- Older comparison checkout: `/Users/harshitsharma/Development/unikey 2`
- Active branch and latest local commit before this audit: `main` at `511bb46`
  (`feat: settings polish with appearance, security, network sections`)
- The active checkout already contained a large uncommitted migration from the
  tracked Flutter desktop client to an untracked React/TypeScript/Tauri client,
  plus extensive Rust, documentation, test, CI, and build-script changes.
- Those pre-existing changes were preserved. No reset, clean, checkout, or
  deletion of unrelated work was performed.
- The pre-existing untracked `apps/desktop/src-tauri/unikey-daemon.bin` was
  not overwritten. It differs from the freshly verified daemon. Native release
  packaging must use `./scripts/build.sh --ui`, which stages a fresh daemon;
  the package verification in this audit was performed from an isolated copy.
- The configured remote is `https://github.com/harshit745/unikey.git`.
  GitHub's repository API, the connected GitHub integration, and `gh repo view`
  all returned repository-not-found during this audit. The public state could
  therefore not be inspected or updated.

## Architecture map

| Layer | Responsibility | Audit status |
|---|---|---|
| `runtime/daemon` | Bootstrap, config, required listeners, service wiring, local API | Working locally; required API/TCP bind failures now stop startup |
| `discovery` | UDP beacon/reply, peer registry, advertised transport endpoint | Verified with two real processes on macOS |
| `transport` | Signed Hello, X25519/HKDF keys, ChaCha20-Poly1305 frames, replay guard, heartbeat | Verified locally; legacy unauthenticated Hello is rejected |
| `pairing` | Short-lived code, request/response correlation, persistent trust | Repaired and verified end to end; trust now pins the transport identity key |
| `services` | Packet routing and service lifecycle | Working; untrusted or key-mismatched sessions are blocked from protected services |
| `files` | Chunked send/receive, filename safety, SHA-256 verification | Verified with an actual transferred file |
| `clipboard` | Poll local clipboard, broadcast changes, suppress loops | Unit-tested; real OS clipboard sync remains unverified |
| `input` | Serialise and inject keyboard/mouse events | Unit-tested; Accessibility/permission behaviour remains unverified |
| Local API | Health, peers, sessions, pairing, transfers, logs, config, SSE | Builds and route tests pass; core pairing/transfer endpoints verified with running daemons |
| React client | Status and feature pages over the localhost API | Production build passes; packaged dashboard reports daemon online; no component tests exist |
| Tauri host | Extract/start daemon, tray, notifications | Six host tests pass; isolated macOS app and DMG build and the packaged app launches |

## Findings corrected in this audit

1. Pairing generated one correlation UUID but placed a different UUID on the
   wire, guaranteeing a timeout.
2. A valid PairRequest updated the responder's trust store but never sent a
   PairResponse to the initiator.
3. Manual pairing connected to the UDP source port instead of the peer's
   advertised TCP port.
4. Pairing could open a duplicate connection even when discovery had already
   established one.
5. The transport accepted a legacy zero-key Hello despite the encryption and
   authentication claims.
6. Trust was based only on a UUID. A new identity key reusing a trusted UUID
   could reach protected services. Trust now pins the authenticated Ed25519
   public key.
7. Cleanup from an older socket could remove a newer replacement session for
   the same peer. Sessions now carry a connection token and stale cleanup is
   conditional.
8. API or TCP bind failure only logged an error while leaving a non-functional
   daemon process alive. These required listeners now fail startup.
9. The README and roadmap described all Phase 1 features and three operating
   systems as complete without matching verification evidence.
10. Phase 2–6 no-op placeholders were registered and shown as "running" in
    the client. Their code remains available for future work, but the MVP
    runtime now advertises only enabled Phase 1 services.

## Verified results

The following were run locally on macOS:

| Command or check | Result |
|---|---|
| `./scripts/test.sh` before focused fixes | 138 Rust tests passed |
| `./scripts/test.sh` after fixes | 141 Rust tests passed |
| Focused pairing/discovery/daemon regression run | 68 tests passed |
| Focused pairing/transport security run | 32 tests passed, including unauthenticated-Hello rejection |
| React `npm run build` | Passed; 2,242 modules transformed |
| Tauri host tests | 6 passed |
| Isolated native package build | Passed; produced `UniKey.app` and an ARM64 DMG without overwriting the pre-existing staged daemon binary |
| Packaged-client launch | Passed; embedded daemon started, UI reported online, and only four enabled Phase 1 services were advertised |
| Isolated daemon smoke test | Startup, health, version, peers API, and Ctrl+C shutdown passed |
| `./scripts/mvp-e2e.sh` | Passed with fail-fast port-collision handling, real discovery, encrypted session, pairing, pinned trust, and file transfer |
| File transfer integrity | Sender and receiver reported `Verified`; content comparison and SHA-256 matched |

CI workflow files exist locally but were not visible in a public repository and
were not executed on GitHub-hosted Windows or Linux runners.

## Prioritised MVP checklist

### P0 — required before recruiter demonstration

- [x] Preserve and audit the existing architecture and uncommitted work.
- [x] Make daemon API/TCP startup fail clearly when required ports cannot bind.
- [x] Verify real UDP discovery and encrypted session establishment.
- [x] Repair PairRequest/PairResponse correlation and rejection behaviour.
- [x] Pin trusted device UUIDs to authenticated Ed25519 identity keys.
- [x] Prevent stale socket cleanup from deleting a newer session.
- [x] Verify one real end-to-end feature: paired file transfer with integrity
  verification.
- [x] Add a reproducible same-host end-to-end script with no mocked network or
  service layer.
- [x] Build and launch an isolated packaged Tauri client and verify its embedded
  daemon reaches the online state.
- [ ] Reproduce pairing and file transfer through the UI rather than curl.
- [ ] Repeat the workflow between two physical devices on the available LAN.
- [ ] Capture one clean screenshot set or short recording of the verified UI
  flow.
- [ ] Resolve the inaccessible GitHub repository before describing the code as
  publicly available.

### P1 — reliability and selection-quality evidence

- [ ] Add React component/integration tests for daemon-down, retry, pairing
  rejection, timeout, transfer failure, and recovery states.
- [ ] Validate clipboard sync between two devices, including OS permissions,
  loop prevention, large content, and image handling.
- [ ] Exercise sleep/wake, network loss, peer expiry, reconnection, and daemon
  restart with persisted trust.
- [ ] Run the committed CI workflows on macOS, Windows, and Linux and record
  actual results.
- [ ] Review key-file permissions and add a documented trust-reset/re-pair
  migration for older trust stores without a pinned key.
- [ ] Add rate limiting or attempt throttling for pairing codes before use on
  untrusted networks.

### P2 — after the MVP is stable

- [ ] Verify keyboard and mouse forwarding with required OS permissions.
- [ ] Improve transfer resume behaviour across process restarts.
- [ ] Add signed release artifacts and a repeatable release checklist.
- [ ] Reassess clipboard as the primary demo feature only after its physical
  two-device behaviour is more reliable than file transfer.
- [ ] Keep Phase 2-6 services explicitly marked as planned; do not register or
  present placeholder services as working product features.

## Current demonstration claim

Accurate: UniKey can currently demonstrate local discovery, an authenticated
and encrypted session, PIN pairing with pinned identity trust, and verified
file transfer between two real daemon processes on macOS.

Not yet accurate: production-ready, independently security-audited, verified
cross-platform, Android/iOS compatible, or fully validated through the packaged
desktop UI.

