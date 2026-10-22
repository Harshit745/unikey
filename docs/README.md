# UniKey Documentation

The UniKey documentation set. Start with **ARCHITECTURE** for the big picture,
then dive into whichever area you need.

## Index

| Document | Audience | What it covers |
|----------|----------|----------------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Everyone | System design, crate layout, startup lifecycle, data flow, event system |
| [NETWORKING.md](NETWORKING.md) | Implementers | Wire protocol: UDP discovery, TCP transport, hello handshake, encrypted framing, message types |
| [ENCRYPTION.md](ENCRYPTION.md) | Implementers, reviewers | Key material, pairing trust, key agreement, cipher suite, replay/rotation protections |
| [API.md](API.md) | GUI/CLI developers | Localhost HTTP API reference, SSE event stream, worked example |
| [DEVELOPER.md](DEVELOPER.md) | Contributors | Repo layout, daily workflow, adding endpoints/services, conventions |
| [TESTING.md](TESTING.md) | Contributors | Running and extending the test suite |
| [SECURITY.md](SECURITY.md) | Reviewers | Security rules, attack surfaces, hardening |
| [BUILD.md](BUILD.md) | Packagers | Build pipeline, flags, release profile, version stamping |
| [RELEASE.md](RELEASE.md) | Maintainers | Release process, versioning, checklist, CI/CD |
| [TROUBLESHOOTING.md](TROUBLESHOOTING.md) | Users | Build and runtime problems, per-platform FAQ |
| [ROADMAP.md](ROADMAP.md) | Everyone | Product phases and future work |

## Also in the repo

- [PRODUCT_VISION.md](../PRODUCT_VISION.md) — what UniKey is and why
- [DESIGN_V2.md](../DESIGN_V2.md) — design rationale for the v2 architecture
- [scripts/README.md](../scripts/README.md) — build/test/release tooling reference
- [config/README.md](../config/README.md) — the shipped configuration
