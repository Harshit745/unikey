use std::convert::Infallible;
use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    http::Method,
    response::{Json, Sse},
    routing::{delete, get, post, put},
};
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

use crate::build_info::BuildInfo;
use crate::config::Config;
use crate::paths::RuntimePaths;
use clipboard::ClipboardService;
use device::DeviceIdentity;
use discovery::peer_manager::{PeerManager, PeerState};
use events::{Envelope, EventBus};
use files::FileTransferService;
use input::event::{InputAction, KeyEvent, MouseEvent};
use pairing::PairingService;
use protocol::{MessageType, Packet};
use services::ServiceRegistry;
use transport::session::SessionManager;
use transport::connection::ConnectionManager;

// ── API port ───────────────────────────────────────────────────────────

/// Port the HTTP API server listens on (localhost only).
pub const API_PORT: u16 = 45895;

// ── Shared state ───────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ApiState {
    pub pairing_service: Arc<PairingService>,
    pub session_manager: SessionManager,
    pub peer_manager: PeerManager,
    pub local_identity: DeviceIdentity,
    pub connection_manager: ConnectionManager,
    /// Registry of every feature service (used by `/services`).
    pub registry: ServiceRegistry,
    /// Build metadata baked into this binary (used by `/version`).
    pub build_info: BuildInfo,
    /// The live configuration. Wrapped in a lock so `PUT /api/v1/config` can
    /// swap it without a restart.
    pub config: Arc<tokio::sync::RwLock<Config>>,
    /// Runtime layout — portable mode flag + active log path (used by `/api/v1/config` and `/api/v1/logs`).
    pub paths: RuntimePaths,
    /// Live event bus published to by services and streamed over SSE.
    pub event_bus: EventBus,
    /// Clipboard service (history + sync status).
    pub clipboard_service: Arc<ClipboardService>,
    /// File transfer service (list / send / cancel).
    pub files_service: Arc<FileTransferService>,
}

// ── Router ─────────────────────────────────────────────────────────────

pub fn build_router(state: ApiState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any);

    Router::new()
        // ── Discovery / metadata (top-level) ────────────────────────
        .route("/", get(root_handler))
        .route("/health", get(health_lite_handler))
        .route("/version", get(version_handler))
        .route("/services", get(services_handler))
        .route("/device", get(device_handler))
        // Health
        .route("/api/v1/health", get(health_handler))
        // Devices (trust store)
        .route("/api/v1/devices", get(list_devices_handler))
        .route("/api/v1/devices/:id", delete(remove_device_handler))
        .route("/api/v1/devices/:id", put(rename_device_handler))
        // Peers (nearby discovered devices)
        .route("/api/v1/peers", get(list_peers_handler))
        // Config & logs (read-only introspection for the desktop GUI)
        .route("/api/v1/config", get(config_handler).put(update_config_handler))
        .route("/api/v1/logs", get(logs_handler))
        // Input forwarding (keyboard & mouse)
        .route("/api/v1/input/keyboard", post(forward_keyboard_handler))
        .route("/api/v1/input/mouse", post(forward_mouse_handler))
        // Pairing
        .route("/api/v1/pairing/generate", post(generate_pairing_handler))
        .route("/api/v1/pairing/status/:session_id", get(pairing_status_handler))
        .route("/api/v1/pairing/initiate", post(initiate_pairing_handler))
        .route("/api/v1/pairing/history", get(pairing_history_handler))
        // Sessions (active encrypted connections)
        .route("/api/v1/sessions", get(list_sessions_handler))
        // Clipboard (history + sync)
        .route("/api/v1/clipboard/history", get(clipboard_history_handler))
        // File transfers
        .route("/api/v1/transfers", get(list_transfers_handler))
        .route("/api/v1/transfers/send", post(send_transfer_handler))
        .route("/api/v1/transfers/cancel", post(cancel_transfer_handler))
        // Live events (SSE + recent history)
        .route("/api/v1/events", get(events_handler))
        .route("/api/v1/events/history", get(event_history_handler))
        // OpenAPI 3.0 machine-readable description of this API.
        .route("/api/v1/openapi.json", get(openapi_handler))
        .layer(cors)
        .with_state(state)
}

// ── Response envelope ──────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: T) -> Json<Self> {
        Json(Self {
            success: true,
            data: Some(data),
            error: None,
        })
    }

    fn err(msg: impl Into<String>) -> Json<Self> {
        Json(Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        })
    }
}

// ── Health ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct HealthStatus {
    status: String,
    version: String,
    device_name: String,
    device_id: Uuid,
    platform: String,
}

async fn health_handler(
    State(state): State<ApiState>,
) -> Json<ApiResponse<HealthStatus>> {
    ApiResponse::ok(HealthStatus {
        status: "running".into(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        device_name: state.local_identity.name.clone(),
        device_id: state.local_identity.id,
        platform: state.local_identity.platform.to_string(),
    })
}

// ── Discovery / metadata ──────────────────────────────────────────────

/// GET / — index of every available endpoint.
async fn root_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "service": "UniKey daemon",
        "version": env!("CARGO_PKG_VERSION"),
        "endpoints": [
            { "method": "GET",    "path": "/",                          "description": "API index" },
            { "method": "GET",    "path": "/health",                    "description": "Liveness probe" },
            { "method": "GET",    "path": "/version",                   "description": "Build & version info" },
            { "method": "GET",    "path": "/services",                  "description": "Registered services & status" },
            { "method": "GET",    "path": "/device",                    "description": "Local device identity" },
            { "method": "GET",    "path": "/api/v1/health",             "description": "Health (envelope)" },
            { "method": "GET",    "path": "/api/v1/devices",            "description": "List trusted devices" },
            { "method": "DELETE", "path": "/api/v1/devices/:id",        "description": "Remove a trusted device" },
            { "method": "PUT",    "path": "/api/v1/devices/:id",        "description": "Rename a trusted device" },
            { "method": "GET",    "path": "/api/v1/peers",              "description": "List discovered LAN peers (name, IP, latency)" },
            { "method": "POST",   "path": "/api/v1/input/keyboard",     "description": "Forward a key event" },
            { "method": "POST",   "path": "/api/v1/input/mouse",        "description": "Forward a mouse event" },
            { "method": "POST",   "path": "/api/v1/pairing/generate",   "description": "Create a pairing session" },
            { "method": "GET",    "path": "/api/v1/pairing/status/:id", "description": "Poll a pairing session" },
            { "method": "POST",   "path": "/api/v1/pairing/initiate",   "description": "Initiate pairing with a peer" },
        ]
    }))
}

/// GET /health — lightweight liveness probe.
async fn health_lite_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

/// GET /version — version/build/platform information.
async fn version_handler(
    State(state): State<ApiState>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": state.build_info.version,
        "build": state.build_info.build,
        "commit": state.build_info.commit,
        "platform": state.build_info.platform,
        "profile": state.build_info.profile,
        "rustc": state.build_info.rustc,
    }))
}

/// GET /services — registered services and their lifecycle status.
async fn services_handler(
    State(state): State<ApiState>,
) -> Json<serde_json::Value> {
    let services: Vec<serde_json::Value> = state
        .registry
        .list_services_with_status()
        .await
        .into_iter()
        .map(|(name, phase, status)| {
            serde_json::json!({
                "name": name,
                "phase": phase,
                "status": status,
            })
        })
        .collect();
    Json(serde_json::json!({ "services": services }))
}

/// GET /device — current local device identity.
async fn device_handler(
    State(state): State<ApiState>,
) -> Json<serde_json::Value> {
    let identity = &state.local_identity;
    Json(serde_json::json!({
        "id": identity.id,
        "name": identity.name,
        "platform": identity.platform.to_string(),
        "device_type": format!("{:?}", identity.device_type),
        "protocol_version": identity.protocol_version,
        "created_at": identity.created_at.to_rfc3339(),
    }))
}

// ── Devices ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct DeviceResponse {
    id: Uuid,
    name: String,
    platform: String,
    alias: Option<String>,
    last_seen: String,
}

async fn list_devices_handler(
    State(state): State<ApiState>,
) -> Json<ApiResponse<Vec<DeviceResponse>>> {
    let devices = state.pairing_service.trusted_devices().await;
    let resp: Vec<DeviceResponse> = devices
        .into_iter()
        .map(|d| DeviceResponse {
            id: d.id,
            name: d.name,
            platform: d.platform,
            alias: d.alias,
            last_seen: d.last_seen.to_rfc3339(),
        })
        .collect();
    ApiResponse::ok(resp)
}

#[derive(Deserialize)]
struct RenameRequest {
    name: String,
}

async fn rename_device_handler(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(req): Json<RenameRequest>,
) -> Json<ApiResponse<()>> {
    match state.pairing_service.rename_trusted_device(&id, &req.name).await {
        Ok(_) => ApiResponse::ok(()),
        Err(e) => ApiResponse::err(e.to_string()),
    }
}

async fn remove_device_handler(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Json<ApiResponse<()>> {
    match state.pairing_service.remove_trusted_device(&id).await {
        Ok(_) => ApiResponse::ok(()),
        Err(e) => ApiResponse::err(e.to_string()),
    }
}

// ── Peers (discovered on LAN) ──────────────────────────────────────────

#[derive(Serialize)]
struct PeerResponse {
    id: Uuid,
    name: String,
    platform: String,
    /// IP address the peer's beacons came from.
    ip: String,
    /// Full UDP socket address of the last beacon/reply.
    address: String,
    /// TCP port the peer listens on for the device transport.
    tcp_port: u16,
    /// Round-trip latency in milliseconds, once measured.
    latency_ms: Option<u64>,
    /// Convenience flag: `state` is `Online` or `Connected`.
    online: bool,
    state: String,
    /// Wall-clock epoch millis of the last packet received from this peer.
    last_seen: u64,
}

async fn list_peers_handler(
    State(state): State<ApiState>,
) -> Json<ApiResponse<Vec<PeerResponse>>> {
    let peers = state.peer_manager.get_all().await;
    let resp: Vec<PeerResponse> = peers
        .into_iter()
        .map(|p| PeerResponse {
            id: p.identity.id,
            name: p.identity.name,
            platform: p.identity.platform.to_string(),
            ip: p.addr.ip().to_string(),
            address: p.addr.to_string(),
            tcp_port: p.tcp_port,
            latency_ms: p.latency_ms,
            online: matches!(p.state, PeerState::Online | PeerState::Connected),
            state: format!("{:?}", p.state),
            last_seen: p.last_seen_unix_ms,
        })
        .collect();
    ApiResponse::ok(resp)
}

// ── Config & logs ─────────────────────────────────────────────────────

/// Read-only snapshot of the loaded daemon configuration plus the resolved
/// runtime layout. Lets the desktop GUI show current ports / portable mode
/// without duplicating daemon path logic.
#[derive(Serialize)]
struct ConfigResponse {
    schema_version: String,
    daemon: crate::config::DaemonConfig,
    network: crate::config::NetworkConfig,
    logging: crate::config::LoggingConfig,
    services: crate::config::ServicesConfig,
    clipboard: crate::config::ClipboardConfig,
    input: crate::config::InputConfig,
    transfer: crate::config::TransferConfig,
    encryption: crate::config::EncryptionConfig,
    /// "portable" or "standard" — where the daemon keeps its state.
    mode: &'static str,
    /// Absolute path to the folder containing the daemon's mutable state.
    root_dir: String,
}

async fn config_handler(State(state): State<ApiState>) -> Json<ApiResponse<ConfigResponse>> {
    let cfg = state.config.read().await;
    ApiResponse::ok(ConfigResponse {
        schema_version: cfg.schema_version.clone(),
        daemon: cfg.daemon.clone(),
        network: cfg.network.clone(),
        logging: cfg.logging.clone(),
        services: cfg.services.clone(),
        clipboard: cfg.clipboard.clone(),
        input: cfg.input.clone(),
        transfer: cfg.transfer.clone(),
        encryption: cfg.encryption.clone(),
        mode: state.paths.mode_name(),
        root_dir: state.paths.root().display().to_string(),
    })
}

/// Body for `PUT /api/v1/config`. Must be the same shape as the config file
/// (schema, daemon, network, logging, services, and the optional feature
/// sections). Replaces the persisted config and re-reads it so the running
/// daemon reflects the new values.
#[derive(Deserialize)]
struct UpdateConfigRequest {
    schema_version: String,
    daemon: crate::config::DaemonConfig,
    network: crate::config::NetworkConfig,
    logging: crate::config::LoggingConfig,
    services: crate::config::ServicesConfig,
    #[serde(default)]
    clipboard: crate::config::ClipboardConfig,
    #[serde(default)]
    input: crate::config::InputConfig,
    #[serde(default)]
    transfer: crate::config::TransferConfig,
    #[serde(default)]
    encryption: crate::config::EncryptionConfig,
}

async fn update_config_handler(
    State(state): State<ApiState>,
    Json(req): Json<UpdateConfigRequest>,
) -> Json<ApiResponse<ConfigResponse>> {
    let new_config = Config {
        schema_version: req.schema_version,
        daemon: req.daemon,
        network: req.network,
        logging: req.logging,
        services: req.services,
        clipboard: req.clipboard,
        input: req.input,
        transfer: req.transfer,
        encryption: req.encryption,
    };

    for warning in new_config.settings_warnings() {
        tracing::warn!("[CONFIG] {warning}");
    }

    let path = state.paths.app_config_file();
    if let Err(e) = new_config.persist(&path) {
        return ApiResponse::err(format!("Failed to save configuration: {e}"));
    }

    // Swap the live copy so the running daemon uses the new settings.
    let mut live = state.config.write().await;
    *live = new_config;
    drop(live);

    config_handler(State(state)).await
}

/// Query params for `/api/v1/logs`.
#[derive(Deserialize)]
struct LogsQuery {
    /// Number of lines to return from the tail of the active log. Default 200.
    #[serde(default = "default_log_tail")]
    tail: usize,
}

fn default_log_tail() -> usize {
    200
}

#[derive(Serialize)]
struct LogsResponse {
    /// Absolute path of the active log file.
    file: String,
    /// The last `tail` lines (oldest → newest).
    lines: Vec<String>,
    /// True if `file` could not be read (e.g. no logs yet).
    truncated: bool,
}

async fn logs_handler(
    State(state): State<ApiState>,
    axum::extract::Query(query): axum::extract::Query<LogsQuery>,
) -> Json<ApiResponse<LogsResponse>> {
    let path = state.paths.active_log_file();
    let tail = query.tail.clamp(1, 5000);

    let content = std::fs::read_to_string(&path);
    let (lines, truncated) = match content {
        Ok(text) => {
            let all: Vec<String> = text.lines().map(str::to_string).collect();
            let len = all.len();
            let skip = len.saturating_sub(tail);
            (all.into_iter().skip(skip).collect(), skip > 0)
        }
        Err(_) => (Vec::new(), false),
    };

    ApiResponse::ok(LogsResponse {
        file: path.display().to_string(),
        lines,
        truncated,
    })
}

// ── Pairing ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct GeneratePairingResponse {
    session_id: Uuid,
    qr_data: String,
    pair_code: String,
    expires_in: u64,
}

async fn generate_pairing_handler(
    State(state): State<ApiState>,
) -> Json<ApiResponse<GeneratePairingResponse>> {
    let session = state
        .pairing_service
        .generate_session(state.local_identity.id)
        .await;
    ApiResponse::ok(GeneratePairingResponse {
        session_id: session.session_id,
        qr_data: session.qr_data.clone(),
        pair_code: session.pair_code.clone(),
        expires_in: session.remaining_seconds(),
    })
}

#[derive(Serialize)]
struct PairingStatusResponse {
    state: String,
    remaining_seconds: u64,
    remote_device: Option<DeviceResponse>,
}

async fn pairing_status_handler(
    State(state): State<ApiState>,
    Path(session_id): Path<Uuid>,
) -> Json<ApiResponse<PairingStatusResponse>> {
    let session = match state.pairing_service.pairing_status(&session_id).await {
        Some(s) => s,
        None => return ApiResponse::err("Session not found"),
    };

    let remote_device = match session.state {
        pairing::PairingState::Accepted(_) | pairing::PairingState::Completed => None,
        _ => None,
    };

    ApiResponse::ok(PairingStatusResponse {
        state: format!("{:?}", session.state),
        remaining_seconds: session.remaining_seconds(),
        remote_device,
    })
}

#[derive(Deserialize)]
struct InitiatePairingRequest {
    /// The 6-digit code displayed on the remote device.
    pair_code: String,
    /// The remote peer's UUID (from the peer list / QR scan).
    peer_id: Uuid,
}

#[derive(Serialize)]
struct InitiatePairingResponse {
    success: bool,
    device_name: Option<String>,
    device_platform: Option<String>,
    error: Option<String>,
}

async fn initiate_pairing_handler(
    State(state): State<ApiState>,
    Json(req): Json<InitiatePairingRequest>,
) -> Json<ApiResponse<InitiatePairingResponse>> {
    // Look up the peer in the peer manager
    let peer = match state.peer_manager.get_peer(&req.peer_id).await {
        Some(p) => p,
        None => return ApiResponse::err("Peer not found in discovered devices"),
    };

    let peer_addr = peer.transport_addr();
    let peer_id = req.peer_id;

    // Build the initiation payload
    let init_session_id = Uuid::new_v4();
    let payload = serde_json::json!({
        "pair_code": req.pair_code,
        "initiator_id": state.local_identity.id.to_string(),
        "initiator_name": state.local_identity.name,
        "initiator_platform": state.local_identity.platform.to_string(),
        "initiation_session_id": init_session_id.to_string(),
    });

    // Create a oneshot channel to receive the PairResponse
    let (tx, rx) = tokio::sync::oneshot::channel();
    state
        .pairing_service
        .register_pending_response(init_session_id, tx)
        .await;

    // Reuse the discovery-established session when present. Opening a second
    // connection for the same peer can race the old session's cleanup and
    // remove the newly inserted session from the manager.
    if !state.session_manager.has_session(&peer_id).await {
        if let Err(e) = state
            .connection_manager
            .connect_to_peer(peer_addr)
            .await
        {
            state
                .pairing_service
                .cancel_pending_response(&init_session_id)
                .await;
            return ApiResponse::err(format!("Failed to connect to peer: {e}"));
        }
    }

    // Send the PairRequest through the session
    if let Some(sender) = state
        .session_manager
        .get_session_sender(&peer_id)
        .await
    {
        let packet = Packet {
            version: 1,
            message_type: MessageType::PairRequest,
            payload: serde_json::to_vec(&payload).unwrap_or_default(),
        };
        if let Err(e) = sender.send(packet).await {
            state
                .pairing_service
                .cancel_pending_response(&init_session_id)
                .await;
            return ApiResponse::err(format!("Failed to send PairRequest: {e}"));
        }
    } else {
        state
            .pairing_service
            .cancel_pending_response(&init_session_id)
            .await;
        return ApiResponse::err("No active session with peer after connection");
    }

    // Wait for the PairResponse (with a 30-second timeout)
    let result = match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
        Ok(Ok(r)) => r,
        Ok(Err(_)) => {
            state
                .pairing_service
                .cancel_pending_response(&init_session_id)
                .await;
            return ApiResponse::err("Pairing response channel closed unexpectedly");
        }
        Err(_) => {
            state
                .pairing_service
                .cancel_pending_response(&init_session_id)
                .await;
            return ApiResponse::err("Pairing timed out — no response within 30 seconds");
        }
    };

    if result.accepted {
        ApiResponse::ok(InitiatePairingResponse {
            success: true,
            device_name: result.peer_name,
            device_platform: result.peer_platform,
            error: None,
        })
    } else {
        ApiResponse::ok(InitiatePairingResponse {
            success: false,
            device_name: None,
            device_platform: None,
            error: Some("Pairing was rejected by the remote device".into()),
        })
    }
}

// ── Input Forwarding ────────────────────────────────────────────

#[derive(Deserialize)]
struct KeyboardForwardRequest {
    /// Target peer UUID.
    peer_id: Uuid,
    /// Which key, e.g. "a", "enter", "shift".
    key: String,
    /// true = pressed, false = released.
    is_press: bool,
    /// Modifier list, e.g. ["ctrl", "shift"].
    #[serde(default)]
    modifiers: Vec<String>,
}

async fn forward_keyboard_handler(
    State(state): State<ApiState>,
    Json(req): Json<KeyboardForwardRequest>,
) -> Json<ApiResponse<()>> {
    let event = if req.is_press {
        KeyEvent::KeyDown {
            key: req.key,
            modifiers: req.modifiers,
        }
    } else {
        KeyEvent::KeyUp {
            key: req.key,
            modifiers: req.modifiers,
        }
    };

    let payload = match serde_json::to_vec(&InputAction::Keyboard(event)) {
        Ok(p) => p,
        Err(e) => return ApiResponse::err(format!("Serialization error: {e}")),
    };

    let packet = Packet {
        version: 1,
        message_type: MessageType::InputKeyEvent,
        payload,
    };

    if let Some(tx) = state.session_manager.get_session_sender(&req.peer_id).await {
        if let Err(e) = tx.send(packet).await {
            return ApiResponse::err(format!("Send failed: {e}"));
        }
        ApiResponse::ok(())
    } else {
        ApiResponse::err("No active session with peer")
    }
}

#[derive(Deserialize)]
struct MouseForwardRequest {
    peer_id: Uuid,
    /// "move", "down", "up", "scroll"
    action: String,
    x: f64,
    y: f64,
    /// Mouse button (1=left, 2=right, 3=middle) — only for down/up actions.
    #[serde(default)]
    button: u8,
    /// Scroll delta — only for scroll action.
    #[serde(default)]
    delta_x: f64,
    #[serde(default)]
    delta_y: f64,
}

async fn forward_mouse_handler(
    State(state): State<ApiState>,
    Json(req): Json<MouseForwardRequest>,
) -> Json<ApiResponse<()>> {
    let event = match req.action.as_str() {
        "move" => MouseEvent::Move { x: req.x, y: req.y },
        "down" => MouseEvent::ButtonDown {
            button: req.button,
            x: req.x,
            y: req.y,
        },
        "up" => MouseEvent::ButtonUp {
            button: req.button,
            x: req.x,
            y: req.y,
        },
        "scroll" => MouseEvent::Scroll {
            delta_x: req.delta_x,
            delta_y: req.delta_y,
        },
        _ => return ApiResponse::err("Unknown mouse action"),
    };

    let payload = match serde_json::to_vec(&InputAction::Mouse(event)) {
        Ok(p) => p,
        Err(e) => return ApiResponse::err(format!("Serialization error: {e}")),
    };

    let packet = Packet {
        version: 1,
        message_type: MessageType::InputMouseEvent,
        payload,
    };

    if let Some(tx) = state.session_manager.get_session_sender(&req.peer_id).await {
        if let Err(e) = tx.send(packet).await {
            return ApiResponse::err(format!("Send failed: {e}"));
        }
        ApiResponse::ok(())
    } else {
        ApiResponse::err("No active session with peer")
    }
}

// ── Sessions (active encrypted connections) ────────────────────────────

async fn list_sessions_handler(
    State(state): State<ApiState>,
) -> Json<ApiResponse<Vec<transport::session::SessionInfo>>> {
    ApiResponse::ok(state.session_manager.list_sessions().await)
}

// ── Clipboard history ──────────────────────────────────────────────────

async fn clipboard_history_handler(
    State(state): State<ApiState>,
) -> Json<ApiResponse<Vec<clipboard::service::ClipboardHistoryEntry>>> {
    ApiResponse::ok(state.clipboard_service.history().await)
}

// ── File transfers ─────────────────────────────────────────────────────

async fn list_transfers_handler(
    State(state): State<ApiState>,
) -> Json<ApiResponse<Vec<files::TransferInfo>>> {
    ApiResponse::ok(state.files_service.list().await)
}

#[derive(Deserialize)]
struct SendTransferRequest {
    /// Which trusted peer receives the file.
    peer_id: Uuid,
    /// Absolute path of the file to send.
    path: String,
}

async fn send_transfer_handler(
    State(state): State<ApiState>,
    Json(req): Json<SendTransferRequest>,
) -> Json<ApiResponse<Uuid>> {
    let transfer_id = match state
        .files_service
        .send_file(&req.peer_id, std::path::Path::new(&req.path))
        .await
    {
        Ok(id) => id,
        Err(e) => return ApiResponse::err(e.to_string()),
    };
    ApiResponse::ok(transfer_id)
}

#[derive(Deserialize)]
struct CancelTransferRequest {
    transfer_id: Uuid,
}

async fn cancel_transfer_handler(
    State(state): State<ApiState>,
    Json(req): Json<CancelTransferRequest>,
) -> Json<ApiResponse<()>> {
    match state.files_service.cancel(req.transfer_id).await {
        Ok(_) => ApiResponse::ok(()),
        Err(e) => ApiResponse::err(e.to_string()),
    }
}

// ── Live events (SSE + recent history) ─────────────────────────────────

/// Server-Sent Events stream. Each line is a JSON `Envelope` with a `type`
/// discriminator (`session_connected`, `clipboard_synced`, …) and a
/// `timestamp`. The GUI keeps a single `EventSource` open to this endpoint
/// and drives the live dashboard from it.
async fn events_handler(
    State(state): State<ApiState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<axum::response::sse::Event, Infallible>>> {
    use axum::response::sse::Event;
    let stream = BroadcastStream::new(state.event_bus.subscribe()).filter_map(|res| match res {
        Ok(env) => Event::default().json_data(env).ok().map(Ok),
        Err(_) => None, // lagged behind a full channel — skip, the history endpoint covers gaps
    });
    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new().interval(std::time::Duration::from_secs(15)),
    )
}

#[derive(Serialize)]
struct EventHistoryResponse {
    events: Vec<Envelope>,
}

async fn event_history_handler(
    State(state): State<ApiState>,
) -> Json<ApiResponse<EventHistoryResponse>> {
    let events = state.event_bus.history(200).await;
    ApiResponse::ok(EventHistoryResponse { events })
}

// ── Pairing history ────────────────────────────────────────────────────

async fn pairing_history_handler(
    State(state): State<ApiState>,
) -> Json<ApiResponse<Vec<pairing::trust_store::PairingEvent>>> {
    ApiResponse::ok(state.pairing_service.pairing_history().await)
}

// ── OpenAPI 3.0 description ────────────────────────────────────────────

/// Machine-readable OpenAPI 3.0 description of the daemon's HTTP API.
/// Serves as the single source of truth for API tooling (Swagger UI, code
/// generators) and is kept in sync with the router by hand.
async fn openapi_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "openapi": "3.0.3",
        "info": {
            "title": "UniKey daemon API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Local HTTP API for the UniKey desktop app. Only binds to loopback. All JSON responses use the envelope { success, data, error }. Live events are streamed over SSE at GET /api/v1/events.",
        },
        "servers": [{ "url": "http://127.0.0.1:45895" }],
        "paths": {
            "/": {
                "get": { "summary": "Root index with endpoint list", "responses": { "200": { "description": "Endpoint index" } } }
            },
            "/health": {
                "get": { "summary": "Liveness check", "responses": { "200": { "description": "ok" } } }
            },
            "/version": {
                "get": { "summary": "Build + version metadata", "responses": { "200": { "description": "Version info" } } }
            },
            "/services": {
                "get": { "summary": "Feature service registry", "responses": { "200": { "description": "Service entries" } } }
            },
            "/device": {
                "get": { "summary": "This device's identity", "responses": { "200": { "description": "Device identity" } } }
            },
            "/api/v1/health": {
                "get": { "summary": "API health (envelope)", "responses": { "200": { "description": "{ success, data: { status } }" } } }
            },
            "/api/v1/devices": {
                "get": { "summary": "List trusted (paired) devices", "responses": { "200": { "description": "Array of trusted devices" } } }
            },
            "/api/v1/devices/{id}": {
                "delete": { "summary": "Remove a trusted device", "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }], "responses": { "200": { "description": "Removed" } } },
                "put": { "summary": "Rename a trusted device", "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }], "responses": { "200": { "description": "Renamed" } } }
            },
            "/api/v1/peers": {
                "get": { "summary": "Nearby discovered devices", "responses": { "200": { "description": "Array of discovered peers" } } }
            },
            "/api/v1/config": {
                "get": { "summary": "Loaded daemon configuration + runtime layout", "responses": { "200": { "description": "Config snapshot" } } }
            },
            "/api/v1/logs": {
                "get": { "summary": "Tail of the active log file", "parameters": [{ "name": "tail", "in": "query", "required": false, "schema": { "type": "integer", "default": 200 } }], "responses": { "200": { "description": "Log lines" } } }
            },
            "/api/v1/input/keyboard": {
                "post": { "summary": "Forward a keyboard event to a peer", "requestBody": { "required": true, "content": { "application/json": { "schema": { "type": "object", "properties": { "peer_id": { "type": "string", "format": "uuid" }, "key": { "type": "string" }, "is_press": { "type": "boolean" }, "modifiers": { "type": "array", "items": { "type": "string" } } } } } } }, "responses": { "200": { "description": "Sent" } } }
            },
            "/api/v1/input/mouse": {
                "post": { "summary": "Forward a mouse event to a peer", "requestBody": { "required": true, "content": { "application/json": { "schema": { "type": "object", "properties": { "peer_id": { "type": "string", "format": "uuid" }, "action": { "type": "string", "enum": ["move", "down", "up", "scroll"] }, "x": { "type": "number" }, "y": { "type": "number" }, "button": { "type": "integer" }, "delta_x": { "type": "number" }, "delta_y": { "type": "number" } } } } } }, "responses": { "200": { "description": "Sent" } } }
            },
            "/api/v1/pairing/generate": {
                "post": { "summary": "Create a pairing session (code + QR)", "responses": { "200": { "description": "session_id, pair_code, qr_data, expires_in" } } }
            },
            "/api/v1/pairing/status/{session_id}": {
                "get": { "summary": "Poll a pairing session's state", "parameters": [{ "name": "session_id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }], "responses": { "200": { "description": "Pairing status" } } }
            },
            "/api/v1/pairing/initiate": {
                "post": { "summary": "Initiate pairing with a peer using its code", "requestBody": { "required": true, "content": { "application/json": { "schema": { "type": "object", "properties": { "pair_code": { "type": "string" }, "peer_id": { "type": "string", "format": "uuid" } } } } } }, "responses": { "200": { "description": "Pairing result" } } }
            },
            "/api/v1/pairing/history": {
                "get": { "summary": "Chronological pairing activity", "responses": { "200": { "description": "Array of pairing events" } } }
            },
            "/api/v1/sessions": {
                "get": { "summary": "Active encrypted sessions", "responses": { "200": { "description": "Array of session infos" } } }
            },
            "/api/v1/clipboard/history": {
                "get": { "summary": "Recent clipboard sync history", "responses": { "200": { "description": "Array of clipboard entries" } } }
            },
            "/api/v1/transfers": {
                "get": { "summary": "List file transfers", "responses": { "200": { "description": "Array of transfers" } } }
            },
            "/api/v1/transfers/send": {
                "post": { "summary": "Send a file to a trusted peer", "requestBody": { "required": true, "content": { "application/json": { "schema": { "type": "object", "properties": { "peer_id": { "type": "string", "format": "uuid" }, "path": { "type": "string" } } } } } }, "responses": { "200": { "description": "New transfer id" } } }
            },
            "/api/v1/transfers/cancel": {
                "post": { "summary": "Cancel an in-flight transfer", "requestBody": { "required": true, "content": { "application/json": { "schema": { "type": "object", "properties": { "transfer_id": { "type": "string", "format": "uuid" } } } } } }, "responses": { "200": { "description": "Cancelled" } } }
            },
            "/api/v1/events": {
                "get": { "summary": "Server-Sent Events stream (text/event-stream)", "responses": { "200": { "description": "Infinite SSE stream of event envelopes" } } }
            },
            "/api/v1/events/history": {
                "get": { "summary": "Recent event envelopes (newest first)", "responses": { "200": { "description": "events array" } } }
            },
            "/api/v1/openapi.json": {
                "get": { "summary": "This OpenAPI document", "responses": { "200": { "description": "OpenAPI 3.0 JSON" } } }
            }
        }
    }))
}

// ── Integration tests ─────────────────────────────────────────────────
//
// These build the real router (with real services) and drive it through
// `tower::ServiceExt::oneshot`, i.e. the full request/response pipeline the
// daemon serves on 127.0.0.1:45895 — no mocks, no separate server needed.

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use chrono::Utc;
    use crypto::IdentityKey;
    use device::{DeviceType, Platform};
    use tower::ServiceExt;

    /// A minimal but realistic `ApiState` for exercising the router.
    fn test_state() -> ApiState {
        let identity = DeviceIdentity {
            id: Uuid::new_v4(),
            name: "test-device".into(),
            platform: Platform::MacOS,
            device_type: DeviceType::Desktop,
            created_at: Utc::now(),
            protocol_version: 1,
            public_key_path: String::new(),
            private_key_path: String::new(),
        };

        let identity_key = IdentityKey::generate();

        let pairing = Arc::new(
            PairingService::with_store_root(None).expect("pairing service builds"),
        );
        let session_manager = SessionManager::new();

        ApiState {
            pairing_service: pairing,
            session_manager: session_manager.clone(),
            peer_manager: PeerManager::new(),
            local_identity: identity.clone(),
            connection_manager: ConnectionManager::new(identity, identity_key, session_manager, None),
            registry: ServiceRegistry::new(),
            build_info: BuildInfo::capture(),
            config: Arc::new(tokio::sync::RwLock::new(crate::config::Config::default())),
            paths: crate::paths::RuntimePaths::detect()
                .expect("runtime paths resolve in test"),
            event_bus: EventBus::new(),
            clipboard_service: Arc::new(ClipboardService::new()),
            files_service: Arc::new(FileTransferService::with_download_dir(
                std::env::temp_dir().join("unikey-test-downloads"),
            )),
        }
    }

    /// Send `GET <path>` through the router and return (status, parsed JSON).
    async fn get_json(path: &str) -> (StatusCode, serde_json::Value) {
        let router = build_router(test_state());
        let response = router
            .oneshot(
                Request::builder()
                    .uri(path)
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router serves request");

        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body reads");
        let json: serde_json::Value =
            serde_json::from_slice(&bytes).expect("response is valid JSON");
        (status, json)
    }

    #[tokio::test]
    async fn root_returns_200_and_endpoint_index() {
        let (status, body) = get_json("/").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["service"], "UniKey daemon");
        assert!(body["endpoints"].is_array());
        assert!(!body["endpoints"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn health_returns_200_ok() {
        let (status, body) = get_json("/health").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "ok");
    }

    #[tokio::test]
    async fn version_returns_200_with_metadata() {
        let (status, body) = get_json("/version").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
        assert!(body["platform"].is_string());
        assert!(body["build"].is_string());
    }

    #[tokio::test]
    async fn services_returns_200_with_entries() {
        let (status, body) = get_json("/services").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body["services"].is_array());
        // Every entry carries name/phase/status.
        for entry in body["services"].as_array().unwrap() {
            assert!(entry["name"].is_string());
            assert!(entry["phase"].is_string());
            assert!(entry["status"].is_string());
        }
    }

    #[tokio::test]
    async fn device_returns_200_with_identity() {
        let (status, body) = get_json("/device").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["name"], "test-device");
        assert!(body["id"].is_string());
        assert!(body["platform"].is_string());
    }

    #[tokio::test]
    async fn legacy_api_health_still_works() {
        let (status, body) = get_json("/api/v1/health").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["status"], "running");
    }

    #[tokio::test]
    async fn config_returns_200_with_layout() {
        let (status, body) = get_json("/api/v1/config").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["schema_version"], "1.0");
        // Network + layout fields present for the Settings page.
        assert!(body["data"]["network"]["api_port"].is_u64());
        assert!(body["data"]["mode"].is_string());
        assert!(body["data"]["root_dir"].is_string());
    }

    #[tokio::test]
    async fn logs_returns_200_lines_array() {
        let (status, body) = get_json("/api/v1/logs").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        assert!(body["data"]["lines"].is_array());
        assert!(body["data"]["file"].is_string());
    }

    #[tokio::test]
    async fn peers_returns_200_with_empty_array() {
        let (status, body) = get_json("/api/v1/peers").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        let data = body["data"].as_array().expect("data is an array");
        assert!(data.is_empty(), "no peers in a fresh test state");
    }

    #[tokio::test]
    async fn sessions_returns_200_with_empty_array() {
        let (status, body) = get_json("/api/v1/sessions").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        let data = body["data"].as_array().expect("data is an array");
        assert!(data.is_empty(), "no sessions in a fresh test state");
    }

    #[tokio::test]
    async fn clipboard_history_returns_200_with_empty_array() {
        let (status, body) = get_json("/api/v1/clipboard/history").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        let data = body["data"].as_array().expect("data is an array");
        assert!(data.is_empty(), "no clipboard history in a fresh test state");
    }

    #[tokio::test]
    async fn transfers_returns_200_with_empty_array() {
        let (status, body) = get_json("/api/v1/transfers").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        let data = body["data"].as_array().expect("data is an array");
        assert!(data.is_empty(), "no transfers in a fresh test state");
    }

    #[tokio::test]
    async fn event_history_returns_200_with_events_array() {
        let (status, body) = get_json("/api/v1/events/history").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        assert!(body["data"]["events"].is_array());
    }

    #[tokio::test]
    async fn pairing_history_returns_200_with_array() {
        let (status, body) = get_json("/api/v1/pairing/history").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["success"], true);
        assert!(body["data"].is_array());
    }

    #[tokio::test]
    async fn openapi_describes_the_api() {
        // The OpenAPI document is served *as* the JSON body (no envelope).
        let state = test_state();
        let router = build_router(state);
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/api/v1/openapi.json")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router serves request");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body reads");
        let spec: serde_json::Value =
            serde_json::from_slice(&bytes).expect("valid JSON");

        assert_eq!(spec["openapi"], "3.0.3");
        assert!(spec["info"]["title"].is_string());
        let paths = spec["paths"].as_object().expect("paths object");
        // Every real endpoint is described.
        for required in [
            "/api/v1/sessions",
            "/api/v1/clipboard/history",
            "/api/v1/transfers",
            "/api/v1/transfers/send",
            "/api/v1/transfers/cancel",
            "/api/v1/events",
            "/api/v1/events/history",
            "/api/v1/pairing/history",
            "/api/v1/input/keyboard",
            "/api/v1/input/mouse",
        ] {
            assert!(paths.contains_key(required), "missing path {required}");
        }
    }

    #[tokio::test]
    async fn config_put_persists_and_live_swaps() {
        // Portable-mode RuntimePaths rooted at a temp dir so the handler
        // writes a throwaway unikey.toml, never the user's real config.
        let base = std::env::temp_dir().join(format!("unikey-test-put-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&base).expect("temp dir");
        std::fs::write(base.join("unikey.portable"), "portable").expect("marker");

        let mut state = test_state();
        state.paths = crate::paths::RuntimePaths::detect_from(&base).expect("portable paths");

        let mut new_cfg = crate::config::Config::default();
        new_cfg.daemon.device_name = Some("renamed".into());
        let body = serde_json::to_vec(&new_cfg).expect("serializes");

        let router = build_router(state);
        let response = router
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/v1/config")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("request builds"),
            )
            .await
            .expect("router serves request");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body reads");
        let json: serde_json::Value =
            serde_json::from_slice(&bytes).expect("response is valid JSON");
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["daemon"]["device_name"], "renamed");

        // Persisted to disk under the temp portable config dir.
        let config_file = base.join("config").join("unikey.toml");
        let persisted = std::fs::read_to_string(&config_file).expect("file written");
        assert!(persisted.contains("renamed"), "persisted config has new name");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn transfers_send_rejects_unknown_peer() {
        // No session exists in a fresh test state, so sending must fail
        // gracefully rather than panic or hang.
        let state = test_state();
        let router = build_router(state);
        let req_body = serde_json::json!({
            "peer_id": Uuid::new_v4(),
            "path": "/tmp/whatever.bin",
        });
        let response = router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/transfers/send")
                    .header("content-type", "application/json")
                    .body(Body::from(req_body.to_string()))
                    .expect("request builds"),
            )
            .await
            .expect("router serves request");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body reads");
        let json: serde_json::Value =
            serde_json::from_slice(&bytes).expect("response is valid JSON");
        // The envelope reports success but the data is an error string.
        assert_eq!(json["success"], false);
        assert!(json["error"].is_string());
    }

    #[tokio::test]
    async fn peers_handler_reports_discovered_fields() {
        // Seed the shared peer manager with one peer and confirm every field
        // the Devices page needs is present in the response.
        use device::{DeviceType, Platform};
        use std::net::SocketAddr;

        let identity = DeviceIdentity {
            id: Uuid::new_v4(),
            name: "neighbour".into(),
            platform: Platform::Windows,
            device_type: DeviceType::Desktop,
            created_at: Utc::now(),
            protocol_version: 1,
            public_key_path: String::new(),
            private_key_path: String::new(),
        };
        let addr: SocketAddr = "192.168.1.42:51987".parse().expect("addr");

        // Seed a fresh test state with one discovered peer.
        let state = test_state();
        state
            .peer_manager
            .add_or_update_peer(identity.clone(), addr, 45892)
            .await;
        state.peer_manager.record_latency(&identity.id, 3).await;

        let router = build_router(state);
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/api/v1/peers")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router serves request");
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body reads");
        let json: serde_json::Value =
            serde_json::from_slice(&bytes).expect("response is valid JSON");

        assert_eq!(json["success"], true);
        let peers = json["data"].as_array().expect("data is an array");
        assert_eq!(peers.len(), 1);
        let peer = &peers[0];
        assert_eq!(peer["name"], "neighbour");
        assert_eq!(peer["platform"], "Windows");
        assert_eq!(peer["ip"], "192.168.1.42");
        assert_eq!(peer["tcp_port"], 45892);
        assert_eq!(peer["latency_ms"], 3);
        assert_eq!(peer["online"], true);
        assert_eq!(peer["state"], "Online");
        assert!(peer["last_seen"].is_u64());
        assert!(peer["id"].is_string());
    }
}
