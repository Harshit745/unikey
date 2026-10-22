# UniKey HTTP API

The daemon exposes a JSON HTTP API on **`127.0.0.1:45895`**. It is bound to localhost only — it is the GUI's control plane and is never meant to be reachable from the LAN.

> A machine-readable **OpenAPI 3.0** description is always available at
> `GET /api/v1/openapi.json` — this document is the human-readable companion.

## Conventions

Every endpoint returns a uniform envelope:

```json
{ "success": true,  "data": { … }, "error": null }
{ "success": false, "data": null,  "error": "human-readable message" }
```

- `data` is a JSON value whose shape depends on the endpoint.
- Errors use a non-2xx status where meaningful, always with the envelope above.
- Unauthenticated by design — the daemon trusts the local process tree. Never expose port 45895 outside the host.

## Endpoints

### System

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | Daemon banner (name, version, docs link) |
| GET | `/health` | Lightweight liveness (`{"status":"ok"}`) |
| GET | `/version` | Version string + build info |
| GET | `/services` | Registered services and their lifecycle status |
| GET | `/device` | This device's identity |
| GET | `/api/v1/health` | Full health (same as `/health` + envelope) |

### Devices & trust

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/devices` | List **trusted** devices (paired previously) |
| PUT | `/api/v1/devices/:id` | Rename a trusted device — body `{ "name": "…" }` |
| DELETE | `/api/v1/devices/:id` | Remove a trusted device (revokes access) |
| GET | `/api/v1/peers` | Live "nearby" registry from discovery (name, platform, IP, RTT, online, last seen) |

### Pairing

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/pairing/generate` | Create a pairing session → `{ session_id, qr_data, pair_code, expires_in }` |
| GET | `/api/v1/pairing/status/:session_id` | Poll a session → `{ state, remaining_seconds, remote_device }` |
| POST | `/api/v1/pairing/initiate` | Enter a remote code → body `{ peer_id, pair_code }`, returns `{ success, device_name, device_platform, error }` |
| GET | `/api/v1/pairing/history` | Past pairing attempts (when, with whom, result) |

Pairing sessions are short-lived (`expires_in` seconds) and rate-limited, so a `pair_code` cannot be brute-forced.

### Sessions

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/sessions` | Active encrypted sessions with peers |

### Clipboard

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/clipboard/history` | Recent clipboard sync entries (direction, content type, preview, peer) |

### File transfers

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/transfers` | Transfer list with status + progress |
| POST | `/api/v1/transfers/send` | Send a file → body `{ peer_id, path }`, returns `transfer_id` |
| POST | `/api/v1/transfers/cancel` | Cancel a transfer → body `{ transfer_id }` |

### Keyboard & mouse forwarding

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/input/keyboard` | Forward a key event → `{ peer_id, key, is_press, modifiers[] }` |
| POST | `/api/v1/input/mouse` | Forward a mouse event → `{ peer_id, action, x, y, button?, delta_x?, delta_y? }` |

`action` is one of `move | down | up | scroll`.

### Events (Server-Sent Events)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/events` | Live event stream (`text/event-stream`) |
| GET | `/api/v1/events/history` | Recent events (bounded) as JSON |

Each SSE frame carries an envelope:

```
data: { "timestamp": "2026-08-07T…", "type": "session_connected",
        "peer_id": "…", "peer_name": "…" }
```

Event `type` values (`snake_case`):

- `service_status`
- `session_connected` / `session_disconnected`
- `clipboard_synced`
- `transfer_started` / `transfer_progress` / `transfer_completed` / `transfer_failed` / `transfer_cancelled`
- `pairing_completed` / `device_added` / `device_removed`

The stream keeps a 15-second keep-alive and reconnects automatically if it drops. The desktop shell uses exactly this stream to drive native notifications.

### Config

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/config` | Full current configuration |
| PUT | `/api/v1/config` | Validate + persist + live-swap a configuration |

### Logs

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/logs` | Recent log lines (bounded ring buffer) |

## Example: pair and send a file

```bash
# 1. Health
curl -s 127.0.0.1:45895/api/v1/health

# 2. Generate a pairing code on device B, enter it on device A
curl -s -X POST 127.0.0.1:45895/api/v1/pairing/generate
curl -s -X POST 127.0.0.1:45895/api/v1/pairing/initiate \
  -H 'content-type: application/json' \
  -d '{"peer_id":"<peer-uuid>","pair_code":"123456"}'

# 3. Wait for the session
curl -s 127.0.0.1:45895/api/v1/sessions

# 4. Send a file
curl -s -X POST 127.0.0.1:45895/api/v1/transfers/send \
  -H 'content-type: application/json' \
  -d '{"peer_id":"<peer-uuid>","path":"/home/me/notes.pdf"}'

# 5. Watch progress live
curl -sN 127.0.0.1:45895/api/v1/events
```

## See Also

- [ARCHITECTURE.md](ARCHITECTURE.md) — the daemon behind the API
- [DEVELOPER.md](DEVELOPER.md) — regenerating the frontend client / extending endpoints
