/**
 * Thin HTTP client for the UniKey daemon local API.
 *
 * Every function returns the parsed JSON body. On network or parse errors
 * the promise rejects so callers can surface the error.
 */

const BASE = 'http://127.0.0.1:45895';
const TIMEOUT_MS = 8_000;

/** Fetch wrapper with a short timeout — keeps the UI feeling snappy. */
async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), TIMEOUT_MS);
  try {
    const res = await fetch(`${BASE}${path}`, { ...init, signal: ctrl.signal });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return (await res.json()) as T;
  } finally {
    clearTimeout(timer);
  }
}

const get = <T,>(path: string) => request<T>(path);

const post = <T,>(path: string, body?: unknown) =>
  request<T>(path, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });

const put = <T,>(path: string, body: unknown) =>
  request<T>(path, {
    method: 'PUT',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  });

const del = <T,>(path: string) => request<T>(path, { method: 'DELETE' });

/** Envelope shape used by all `/api/v1/*` endpoints: `{ success, data, error }`. */
interface ApiEnvelope<T> {
  success: boolean;
  data: T | null;
  error: string | null;
}

/** Fetch an `/api/v1/*` endpoint and unwrap its `data` payload. */
async function getEnvelope<T>(path: string): Promise<T> {
  const res = await get<ApiEnvelope<T>>(path);
  if (!res.success || res.data === null) throw new Error(res.error ?? 'API error');
  return res.data;
}

/** POST an `/api/v1/*` endpoint and unwrap its `data` payload. */
async function postEnvelope<T>(path: string, body?: unknown): Promise<T> {
  const res = await post<ApiEnvelope<T>>(path, body);
  if (!res.success || res.data === null) throw new Error(res.error ?? 'API error');
  return res.data;
}

/** PUT an `/api/v1/*` endpoint and unwrap its `data` payload. */
async function putEnvelope<T>(path: string, body: unknown): Promise<T> {
  const res = await put<ApiEnvelope<T>>(path, body);
  if (!res.success || res.data === null) throw new Error(res.error ?? 'API error');
  return res.data;
}

/** DELETE an `/api/v1/*` endpoint and unwrap its `data` payload. */
async function deleteEnvelope<T>(path: string): Promise<T> {
  const res = await del<ApiEnvelope<T>>(path);
  if (!res.success || res.data === null) throw new Error(res.error ?? 'API error');
  return res.data;
}

// ── Endpoints ─────────────────────────────────────────────────────────

export const api = {
  health:    () => get<import('./types').HealthStatus>('/health'),
  version:   () => get<import('./types').VersionInfo>('/version'),
  device:    () => get<import('./types').DeviceInfo>('/device'),
  root:      () => get<import('./types').RootInfo>('/'),
  services:  () => get<import('./types').ServicesResponse>('/services'),
  config:    () => getEnvelope<import('./types').ConfigResponse>('/api/v1/config'),
  updateConfig: (cfg: import('./types').ConfigResponse) =>
    putEnvelope<import('./types').ConfigResponse>('/api/v1/config', cfg),
  logs:      (tail = 200) => getEnvelope<import('./types').LogsResponse>(`/api/v1/logs?tail=${tail}`),
  peers:     () => getEnvelope<import('./types').PeerInfo[]>('/api/v1/peers'),
  sessions:  () => getEnvelope<import('./types').SessionInfo[]>('/api/v1/sessions'),
  trustedDevices: () => getEnvelope<import('./types').TrustedDevice[]>('/api/v1/devices'),
  removeDevice: (id: string) => deleteEnvelope<null>(`/api/v1/devices/${id}`),
  renameDevice: (id: string, name: string) =>
    putEnvelope<null>(`/api/v1/devices/${id}`, { name }),

  // ── Clipboard ────────────────────────────────────────────────────
  clipboardHistory: () =>
    getEnvelope<import('./types').ClipboardEntry[]>('/api/v1/clipboard/history'),

  // ── Transfers ────────────────────────────────────────────────────
  transfers: () => getEnvelope<import('./types').TransferInfo[]>('/api/v1/transfers'),
  sendTransfer: (peerId: string, path: string) =>
    postEnvelope<string>('/api/v1/transfers/send', { peer_id: peerId, path }),
  cancelTransfer: (transferId: string) =>
    postEnvelope<null>('/api/v1/transfers/cancel', { transfer_id: transferId }),

  // ── Pairing ──────────────────────────────────────────────────────
  pairingGenerate: () =>
    postEnvelope<import('./types').PairingGenerateResponse>('/api/v1/pairing/generate'),
  pairingStatus: (sessionId: string) =>
    getEnvelope<import('./types').PairingStatusResponse>(`/api/v1/pairing/status/${sessionId}`),
  pairingInitiate: (pairCode: string, peerId: string) =>
    postEnvelope<import('./types').PairingInitiateResponse>('/api/v1/pairing/initiate', {
      pair_code: pairCode,
      peer_id: peerId,
    }),
  pairingHistory: () =>
    getEnvelope<import('./types').PairingEvent[]>('/api/v1/pairing/history'),

  // ── Events ───────────────────────────────────────────────────────
  eventHistory: () =>
    getEnvelope<import('./types').EventHistoryResponse>('/api/v1/events/history'),
  /** Open an SSE stream to `/api/v1/events`. Returns an EventSource. */
  events: () => new EventSource(`${BASE}/api/v1/events`),

  // ── Input forwarding ─────────────────────────────────────────────
  cursorStatus: () => getEnvelope<import('./types').CursorStatus>('/api/v1/input/cursor'),
  cursorStart: (peerId: string, edge: 'left' | 'right') =>
    postEnvelope<import('./types').CursorStatus>('/api/v1/input/cursor/start', { peer_id: peerId, edge }),
  cursorStop: () => postEnvelope<import('./types').CursorStatus>('/api/v1/input/cursor/stop'),
  cursorReceiving: (enabled: boolean) =>
    postEnvelope<import('./types').CursorStatus>('/api/v1/input/cursor/receiving', { enabled }),
  forwardKeyboard: (peerId: string, key: string, isPress: boolean, modifiers: string[] = []) =>
    postEnvelope<null>('/api/v1/input/keyboard', {
      peer_id: peerId, key, is_press: isPress, modifiers,
    }),
  forwardMouse: (peerId: string, action: string, x = 0, y = 0, opts: { button?: number; delta_x?: number; delta_y?: number } = {}) =>
    postEnvelope<null>('/api/v1/input/mouse', {
      peer_id: peerId, action, x, y, ...opts,
    }),
};

/** Subscribe to the daemon's live event stream with automatic reconnect.
 *  `onEvent` receives each parsed envelope; the promise resolves when the
 *  source is closed. */
export function subscribeEvents(
  onEvent: (env: import('./types').Envelope) => void,
  onOpen?: () => void,
): () => void {
  const es = api.events();
  es.onopen = () => onOpen?.();
  es.onmessage = (e) => {
    try { onEvent(JSON.parse(e.data)); } catch { /* ignore malformed frames */ }
  };
  return () => es.close();
}
