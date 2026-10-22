# UniKey Networking

This document describes everything that goes over the wire: UDP discovery, the TCP transport, the hello handshake, encrypted framing, and the localhost HTTP API. It is the companion to [ENCRYPTION.md](ENCRYPTION.md), which covers the cryptography.

## Ports

| Port | Protocol | Purpose |
|------|----------|---------|
| 45892 | TCP | Encrypted device transport (sessions) |
| 45893 | UDP | Discovery beacons / replies |
| 45895 | TCP | Localhost HTTP API (GUI ⇄ daemon only, never exposed) |

All three are configurable via the `network` config section. Peers on a LAN must share the same TCP and UDP ports to interoperate.

## Device Discovery (UDP)

Every device broadcasts a JSON `DiscoveryPacket` to `255.255.255.255:45893` every **2 seconds**:

```json
{
  "protocol_version": 1,
  "device_id": "…",          // UUIDv4, stable per device
  "device_name": "…",
  "platform": "macOS",
  "tcp_port": 45892,
  "sent_at_ms": 1720000000000
}
```

On receipt a device:

1. Ignores beacons from itself (`device_id` matches its own).
2. Records the peer (name, platform, source IP, advertised TCP port) in the peer registry and measures round-trip time by replying.
3. If the peer is a **trusted** (previously paired) device, the transport attempts a TCP connection to it.

Peers that stop beaconing are marked offline after a grace period and pruned from the registry entirely once their `offline_at` age exceeds the prune threshold, so the GUI's "nearby devices" list stays live.

Discovery is **identity-only and unauthenticated by design**: it must be, so unknown devices can find each other to begin pairing. Nothing security-sensitive is ever carried in a beacon, and no inbound TCP connection from an untrusted device is accepted without a handshake.

## TCP Transport

The transport listener accepts connections on 45892. Each accepted socket runs the connection loop:

- **Length-prefixed frames** — every frame is preceded by a 4-byte big-endian length.
- **Handshake first** — a connection that does not complete the hello handshake is closed within a timeout.
- **Heartbeat** — idle connections are kept alive with Ping/Pong so half-open TCP connections are detected.
- **Reconnect** — the client side re-establishes the session on an unexpected drop.
- **Idle reaping** — sessions with no traffic beyond the idle timeout are torn down and their peers marked offline.

### Hello handshake

Before any application data flows, both sides exchange a `Hello` carrying:

- protocol version
- sender device id, name, platform
- the sender's **Ed25519 public key**
- an **X25519 public key** (ephemeral per handshake)
- a signature over the whole hello by the Ed25519 key

Both sides verify the signature against the peer's **known** identity (from the trust store for a previously paired device, or the pairing exchange for a first contact). The handshake also binds the X25519 keys, so a MITM cannot substitute their own key without failing the Ed25519 check.

### Encrypted frame layout

After the handshake, every application packet is wrapped in an authenticated-encrypted frame:

```
 0     1        2          10         22
 +-----+--------+----------+----------+------------------------------+
 | ver | flags  | counter  | nonce    | ciphertext (ChaCha20-Poly1305)|
 | (1) | (1)    | (8, BE)  | (12)     |   = plaintext || 16-byte tag  |
 +-----+--------+----------+----------+------------------------------+
 \──────────── AAD (10 bytes) ─────────/
```

- The `counter` is monotonic per direction and participates in the replay guard **and** the nonce, so the same (key, counter) can never be reused.
- The 10-byte AAD binds the header to the ciphertext, preventing header rewriting.
- The inner plaintext is the normal packet (version | type | length | payload), so message framing rules are unchanged — only the outer wrapping is encrypted.

## Message Types

| Type | Code | Purpose |
|------|------|---------|
| Hello | 1 | Handshake |
| Ping / Pong | 2 / 3 | Heartbeat |
| PairRequest / PairResponse | 4 / 5 | Pairing flow |
| Clipboard | 6 | Synced clipboard content |
| FileMetadata / FileChunk | 7 / 8 | Transfer header / payload |
| Disconnect | 9 | Graceful teardown |
| InputKeyEvent / InputMouseEvent | 10 / 11 | Keyboard / mouse forwarding |
| FileAck / FileCancel | 12 / 13 | Transfer verification / abort |
| InputGrab / InputRelease | 14 / 15 | Shared-cursor ownership |

Unknown type codes are rejected at decode time — no malformed packet reaches a service.

## HTTP API (localhost)

The daemon serves a JSON API on `127.0.0.1:45895` only. All endpoints return the envelope `{ "success": true, "data": …, "error": null }` (or `success: false` with a message). See [API.md](API.md) for the full reference, including the live SSE stream and the generated OpenAPI 3.0 document at `GET /api/v1/openapi.json`.

## See Also

- [ENCRYPTION.md](ENCRYPTION.md) — key exchange, cipher suite, replay protection
- [ARCHITECTURE.md](ARCHITECTURE.md) — how the pieces fit together
