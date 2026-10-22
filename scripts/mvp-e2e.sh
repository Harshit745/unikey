#!/usr/bin/env bash
# Real same-host MVP verification: two daemon processes discover, connect,
# pair, persist trust, and transfer a file through the encrypted transport.
# No service or network layer is mocked.
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DAEMON="$ROOT/target/debug/daemon"
BASE_PORT="${UNIKEY_E2E_BASE_PORT:-47890}"
TCP_A="$BASE_PORT"
DISCOVERY_PORT="$((BASE_PORT + 1))"
API_A="$((BASE_PORT + 3))"
TCP_B="$((BASE_PORT + 10))"
API_B="$((BASE_PORT + 13))"

for command_name in cargo curl python3 cmp; do
  command -v "$command_name" >/dev/null 2>&1 || {
    printf '%s\n' "[error] required command not found: $command_name" >&2
    exit 1
  }
done

cargo build --manifest-path "$ROOT/Cargo.toml" -p daemon >/dev/null

RUN_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/unikey-mvp-e2e.XXXXXX")"
A_DIR="$RUN_ROOT/a"
B_DIR="$RUN_ROOT/b"
C_DIR="$RUN_ROOT/port-conflict"
mkdir -p "$A_DIR/config" "$A_DIR/logs" "$B_DIR/config" "$B_DIR/logs" \
  "$C_DIR/config" "$C_DIR/logs"
cp "$DAEMON" "$A_DIR/daemon"
cp "$DAEMON" "$B_DIR/daemon"
cp "$DAEMON" "$C_DIR/daemon"
touch "$A_DIR/unikey.portable" "$B_DIR/unikey.portable" \
  "$C_DIR/unikey.portable"

write_config() {
  local dir="$1" name="$2" tcp_port="$3" api_port="$4"
  cat >"$dir/config/unikey.toml" <<EOF
schema_version = "1.0"

[daemon]
device_name = "$name"

[network]
tcp_bind = "0.0.0.0"
tcp_port = $tcp_port
discovery_port = $DISCOVERY_PORT
api_port = $api_port
discovery_bind = "0.0.0.0"

[logging]
level = "info"
max_bytes = 5242880
max_backups = 2
file_enabled = true

[services]
clipboard = false
input = false
files = true
pairing = true

[clipboard]
auto_sync = false
sync_images = false
history_limit = 20
poll_interval_ms = 500

[input]
keyboard_forwarding = false
mouse_forwarding = false
edge_switching = false

[transfer]
max_file_size = 10485760
download_dir = "$dir/downloads"

[encryption]
key_rotation_minutes = 0
max_replay_window_ms = 30000
require_pairing = true
EOF
}

write_config "$A_DIR" "UniKey-E2E-A" "$TCP_A" "$API_A"
write_config "$B_DIR" "UniKey-E2E-B" "$TCP_B" "$API_B"
write_config "$C_DIR" "UniKey-E2E-Port-Conflict" "$TCP_A" "$API_A"

A_PID=""
B_PID=""
cleanup() {
  [ -z "$A_PID" ] || kill -INT "$A_PID" 2>/dev/null || true
  [ -z "$B_PID" ] || kill -INT "$B_PID" 2>/dev/null || true
  [ -z "$A_PID" ] || wait "$A_PID" 2>/dev/null || true
  [ -z "$B_PID" ] || wait "$B_PID" 2>/dev/null || true
  rm -rf "$RUN_ROOT"
}
trap cleanup EXIT INT TERM

(cd "$A_DIR" && exec ./daemon >daemon.stdout.log 2>&1) &
A_PID="$!"
(cd "$B_DIR" && exec ./daemon >daemon.stdout.log 2>&1) &
B_PID="$!"

wait_for_url() {
  local url="$1"
  local attempt
  for ((attempt = 0; attempt < 50; attempt++)); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.2
  done
  printf '%s\n' "[error] timed out waiting for $url" >&2
  return 1
}

wait_for_json_match() {
  local url="$1" expression="$2"
  local attempt
  for ((attempt = 0; attempt < 75; attempt++)); do
    if curl -fsS "$url" | python3 -c \
      "import json,sys; value=json.load(sys.stdin); raise SystemExit(0 if ($expression) else 1)"; then
      return 0
    fi
    sleep 0.2
  done
  printf '%s\n' "[error] condition did not become true for $url" >&2
  return 1
}

wait_for_url "http://127.0.0.1:$API_A/health"
wait_for_url "http://127.0.0.1:$API_B/health"

# A required API/TCP port collision must stop startup instead of leaving a
# process running without its control plane or transport listener.
set +e
(cd "$C_DIR" && ./daemon >daemon.stdout.log 2>&1)
CONFLICT_STATUS="$?"
set -e
if [ "$CONFLICT_STATUS" -eq 0 ]; then
  printf '%s\n' '[error] daemon unexpectedly succeeded with occupied ports' >&2
  exit 1
fi

wait_for_json_match "http://127.0.0.1:$API_A/api/v1/peers" "len(value.get('data') or []) == 1"
wait_for_json_match "http://127.0.0.1:$API_A/api/v1/sessions" "len(value.get('data') or []) == 1"

B_ID="$(curl -fsS "http://127.0.0.1:$API_B/api/v1/health" | python3 -c \
  "import json,sys; print(json.load(sys.stdin)['data']['device_id'])")"
PAIR_JSON="$(curl -fsS -X POST "http://127.0.0.1:$API_B/api/v1/pairing/generate")"
PAIR_CODE="$(printf '%s' "$PAIR_JSON" | python3 -c \
  "import json,sys; print(json.load(sys.stdin)['data']['pair_code'])")"

PAIR_BODY="$(python3 -c \
  "import json; print(json.dumps({'pair_code':'$PAIR_CODE','peer_id':'$B_ID'}))")"
PAIR_RESULT="$(curl -fsS -X POST -H 'content-type: application/json' \
  --data "$PAIR_BODY" "http://127.0.0.1:$API_A/api/v1/pairing/initiate")"
printf '%s' "$PAIR_RESULT" | python3 -c \
  "import json,sys; value=json.load(sys.stdin); assert value['success'] and value['data']['success']"

printf '%s\n' 'UniKey verified MVP file transfer' >"$A_DIR/unikey-mvp-demo.txt"
TRANSFER_BODY="$(python3 -c \
  "import json; print(json.dumps({'peer_id':'$B_ID','path':'$A_DIR/unikey-mvp-demo.txt'}))")"
curl -fsS -X POST -H 'content-type: application/json' --data "$TRANSFER_BODY" \
  "http://127.0.0.1:$API_A/api/v1/transfers/send" >/dev/null

for ((attempt = 0; attempt < 50; attempt++)); do
  if [ -f "$B_DIR/downloads/unikey-mvp-demo.txt" ]; then
    break
  fi
  sleep 0.1
done
[ -f "$B_DIR/downloads/unikey-mvp-demo.txt" ] || {
  printf '%s\n' '[error] receiver did not create the transferred file' >&2
  exit 1
}
cmp "$A_DIR/unikey-mvp-demo.txt" "$B_DIR/downloads/unikey-mvp-demo.txt"
wait_for_json_match "http://127.0.0.1:$API_A/api/v1/transfers" \
  "len(value.get('data') or []) == 1 and value['data'][0]['status'] == 'Verified'"
wait_for_json_match "http://127.0.0.1:$API_B/api/v1/transfers" \
  "len(value.get('data') or []) == 1 and value['data'][0]['status'] == 'Verified'"

printf '%s\n' '[ok] discovery, encrypted session, pairing, pinned trust, and file transfer verified'
