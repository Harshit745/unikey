/** Typed shapes for every daemon endpoint we consume. */

export interface HealthStatus {
  status: 'ok' | 'down';
  endpoint?: string;
}

export interface VersionInfo {
  version: string;
  build: string;
  commit: string;
  platform: string;
  profile: string;
  rustc: string;
}

export interface DeviceInfo {
  id: string;
  name: string;
  platform: string;
  device_type: string;
  protocol_version: string;
  created_at: string;
}

export interface ServiceStatus {
  name: string;
  phase: string;
  status: string; // "registered" | "running" | "stopped"
}

export interface ServicesResponse {
  services: ServiceStatus[];
}

/** Shape of `/api/v1/config` `data` — mirrors the daemon's `Config`. */
export interface ConfigResponse {
  schema_version: string;
  daemon: { device_name: string | null };
  network: {
    tcp_bind: string;
    tcp_port: number;
    discovery_port: number;
    api_port: number;
    discovery_bind: string;
  };
  logging: { level: string; max_bytes: number; max_backups: number; file_enabled: boolean };
  services: { clipboard: boolean; input: boolean; files: boolean; pairing: boolean };
  clipboard: {
    auto_sync: boolean;
    sync_images: boolean;
    history_limit: number;
    poll_interval_ms: number;
  };
  input: {
    keyboard_forwarding: boolean;
    mouse_forwarding: boolean;
    edge_switching: boolean;
  };
  transfer: {
    max_file_size: number;
    download_dir: string | null;
  };
  encryption: {
    key_rotation_minutes: number;
    max_replay_window_ms: number;
    require_pairing: boolean;
  };
  mode: string;
  root_dir: string;
}

/** A device auto-discovered on the LAN (from `/api/v1/peers`). */
export interface PeerInfo {
  id: string;
  name: string;
  platform: string;
  /** IP address the peer's beacons came from. */
  ip: string;
  /** Full UDP socket address of the last beacon/reply. */
  address: string;
  tcp_port: number;
  /** Round-trip latency in milliseconds, once measured. */
  latency_ms: number | null;
  online: boolean;
  state: string;
  /** Wall-clock epoch millis of the last packet from this peer. */
  last_seen: number;
}

export interface LogsResponse {
  file: string;
  lines: string[];
  truncated: boolean;
}

export interface EndpointInfo {
  method: string;
  path: string;
  description: string;
}

export interface RootInfo {
  service: string;
  version: string;
  endpoints: EndpointInfo[];
}

/** One active encrypted session (from `/api/v1/sessions`). */
export interface SessionInfo {
  peer_id: string;
  peer_name: string;
  platform: string;
  idle_for_ms: number;
  rtt_ms: number | null;
}

/** A trusted (previously paired) device. */
export interface TrustedDevice {
  id: string;
  name: string;
  platform: string;
  alias: string | null;
  last_seen: string;
}

/** One clipboard history entry (from `/api/v1/clipboard/history`). */
export interface ClipboardEntry {
  direction: string;
  peer_id: string | null;
  content_type: string;
  preview: string;
  timestamp: string;
}

/** Transfer state (from `/api/v1/transfers`). */
export interface TransferInfo {
  transfer_id: string;
  peer_id: string;
  filename: string;
  total_size: number;
  transferred: number;
  direction: string;
  status: string;
  progress_percent: number;
  error: string | null;
  started_at: string | null;
  completed_at: string | null;
}

/** Response of `POST /api/v1/pairing/generate`. */
export interface PairingGenerateResponse {
  session_id: string;
  qr_data: string;
  pair_code: string;
  expires_in: number;
}

/** Response of `GET /api/v1/pairing/status/:id`. */
export interface PairingStatusResponse {
  state: string;
  remaining_seconds: number;
  remote_device: TrustedDevice | null;
}

/** Response of `POST /api/v1/pairing/initiate`. */
export interface PairingInitiateResponse {
  success: boolean;
  device_name: string | null;
  device_platform: string | null;
  error: string | null;
}

/** One entry in the pairing history log. */
export interface PairingEvent {
  kind: string;
  peer_id: string;
  peer_name: string;
  timestamp: string;
}

/** A single event envelope streamed over SSE. */
export interface Envelope {
  timestamp: string;
  /** Discriminator — e.g. "session_connected", "transfer_progress". */
  type: string;
  [key: string]: unknown;
}

/** Response of `/api/v1/events/history`. */
export interface EventHistoryResponse {
  events: Envelope[];
}
