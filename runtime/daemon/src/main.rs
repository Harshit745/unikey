//! UniKey daemon — orchestrates discovery, transport, services, and the
//! HTTP API that the desktop UI talks to.
//!
//! Bootstrap order matters:
//!   1. Resolve runtime paths (portable folder vs OS config dir).
//!   2. Create every directory the daemon needs (config, logs, keys, assets).
//!   3. Initialise logging (console + rotating file + panic hook).
//!   4. Load (or create) `unikey.toml` configuration.
//!   5. Load-or-create the device identity and keypair.
//!   6. Start all services and listeners.
//!
//! The daemon never crashes because config, logs, or an identity are missing —
//! every one of those is auto-created with safe defaults.

// On Windows, release builds run without a console window (double-click
// launch). Logging still reaches `logs/latest.log`. Debug builds keep the
// console for development.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![recursion_limit = "256"] // OpenAPI's expanded endpoint schema.

use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::net::{TcpListener, UdpSocket};
use tokio::signal;
use tokio::sync::mpsc;
use tokio::time;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use device::DeviceManager;
use discovery::peer_manager::PeerManager;
use events::EventBus;
use protocol::{MessageType, Packet};
use services::{Phase, Service, ServiceContext, ServiceRegistry};

use transport::connection::ConnectionManager;
use transport::session::SessionManager;

// ---------------------------------------------------------------------------
// Phase 1 — Core services
// ---------------------------------------------------------------------------
use pairing::PairingService;
use clipboard::ClipboardService;
use input::InputService;
use files::FileTransferService;

// ---------------------------------------------------------------------------
// Local modules
// ---------------------------------------------------------------------------
mod api;
mod build_info;
mod config;
mod logging;
mod paths;

use config::Config;
use paths::RuntimePaths;

/// Newtype wrapper so the shared `Arc<PairingService>` can implement `Service`.
#[derive(Debug)]
struct ArcPairingService(Arc<PairingService>);

#[async_trait]
impl Service for ArcPairingService {
    fn name(&self) -> &'static str {
        self.0.name()
    }
    fn phase(&self) -> Phase {
        self.0.phase()
    }
    async fn start(&self, ctx: ServiceContext) -> Result<()> {
        self.0.start(ctx).await
    }
    async fn stop(&self) -> Result<()> {
        self.0.stop().await
    }
    async fn handle_packet(
        &self,
        peer_id: &Uuid,
        msg_type: MessageType,
        payload: &[u8],
    ) -> Result<()> {
        self.0.handle_packet(peer_id, msg_type, payload).await
    }
}

/// Newtype wrapper so the shared `Arc<ClipboardService>` can implement `Service`.
#[derive(Debug)]
struct ArcClipboardService(Arc<ClipboardService>);

#[async_trait]
impl Service for ArcClipboardService {
    fn name(&self) -> &'static str {
        self.0.name()
    }
    fn phase(&self) -> Phase {
        self.0.phase()
    }
    async fn start(&self, ctx: ServiceContext) -> Result<()> {
        self.0.start(ctx).await
    }
    async fn stop(&self) -> Result<()> {
        self.0.stop().await
    }
    async fn handle_packet(
        &self,
        peer_id: &Uuid,
        msg_type: MessageType,
        payload: &[u8],
    ) -> Result<()> {
        self.0.handle_packet(peer_id, msg_type, payload).await
    }
}

/// Newtype wrapper so the shared `Arc<FileTransferService>` can implement `Service`.
#[derive(Debug)]
struct ArcFileTransferService(Arc<FileTransferService>);

#[async_trait]
impl Service for ArcFileTransferService {
    fn name(&self) -> &'static str {
        self.0.name()
    }
    fn phase(&self) -> Phase {
        self.0.phase()
    }
    async fn start(&self, ctx: ServiceContext) -> Result<()> {
        self.0.start(ctx).await
    }
    async fn stop(&self) -> Result<()> {
        self.0.stop().await
    }
    async fn handle_packet(
        &self,
        peer_id: &Uuid,
        msg_type: MessageType,
        payload: &[u8],
    ) -> Result<()> {
        self.0.handle_packet(peer_id, msg_type, payload).await
    }
}

// ---------------------------------------------------------------------------
/// Bind the UDP discovery socket with broadcast + address reuse.
///
/// Reuse (`SO_REUSEADDR`, and `SO_REUSEPORT` on unix) lets a second UniKey
/// instance on the same host share the discovery port — otherwise that
/// instance could never participate in discovery on its own machine.
fn build_udp_socket(bind: &str, port: u16) -> anyhow::Result<tokio::net::UdpSocket> {
    let addr: SocketAddr = format!("{bind}:{port}").parse()?;
    let socket = socket2::Socket::new(
        socket2::Domain::IPV4,
        socket2::Type::DGRAM,
        Some(socket2::Protocol::UDP),
    )?;
    socket.set_reuse_address(true)?;
    #[cfg(unix)]
    socket.set_reuse_port(true)?;
    socket.set_broadcast(true)?;
    socket.set_nonblocking(true)?;
    socket.bind(&addr.into())?;
    let std_socket: std::net::UdpSocket = socket.into();
    Ok(tokio::net::UdpSocket::from_std(std_socket)?)
}

/// Current wall clock in epoch milliseconds (beacon timestamps).
fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Run the UDP discovery loop: broadcast this device every 2 s, and process
/// inbound beacons/replies (record peers, measure RTT, auto-connect TCP).
///
/// All eight parameters are independent collaborators of one long-lived task;
/// grouping them would just move the ceremony to a call site.
#[allow(clippy::too_many_arguments)]
async fn run_udp_discovery(
    socket: Arc<UdpSocket>,
    local_identity: device::DeviceIdentity,
    discovery_port: u16,
    local_tcp_port: u16,
    peer_manager: discovery::peer_manager::PeerManager,
    connection_manager: transport::connection::ConnectionManager,
    session_manager: transport::session::SessionManager,
    shutdown: CancellationToken,
) {
    // The beacon we broadcast; `sent_at_ms` is refreshed on every tick.
    let beacon = discovery::DiscoveryPacket::beacon(
        common::constants::PROTOCOL_VERSION,
        local_identity.id,
        local_identity.name.clone(),
        local_identity.platform.to_string(),
        local_tcp_port,
        0,
    );

    // ── Broadcast task ─────────────────────────────────────────────────
    let broadcast_socket = socket.clone();
    let broadcast_shutdown = shutdown.clone();
    let broadcast_beacon = beacon.clone();
    tokio::spawn(async move {
        let target = format!("255.255.255.255:{discovery_port}");
        let mut interval = time::interval(Duration::from_secs(2));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let mut pkt = broadcast_beacon.clone();
                    pkt.sent_at_ms = now_unix_ms();
                    let bytes = match serde_json::to_vec(&pkt) {
                        Ok(b) => b,
                        Err(_) => continue,
                    };
                    if let Err(e) = broadcast_socket.send_to(&bytes, &target).await {
                        tracing::warn!("[UDP] Broadcast to {target} failed: {e}");
                    }
                }
                _ = broadcast_shutdown.cancelled() => break,
            }
        }
    });

    // ── Receive loop ───────────────────────────────────────────────────
    let mut buf = [0u8; 4096];
    loop {
        tokio::select! {
            result = socket.recv_from(&mut buf) => {
                let (len, addr) = match result {
                    Ok(pair) => pair,
                    Err(e) => {
                        tracing::warn!("[UDP] Receive error: {e}");
                        continue;
                    }
                };

                let packet: discovery::DiscoveryPacket = match serde_json::from_slice(&buf[..len]) {
                    Ok(p) => p,
                    Err(_) => continue, // not a UniKey beacon
                };
                if packet.device_id == local_identity.id {
                    continue; // our own broadcast/reply — ignore
                }

                // Record the peer (identity, source IP, advertised TCP port).
                let identity = device::DeviceIdentity {
                    id: packet.device_id,
                    name: packet.device_name.clone(),
                    platform: device::Platform::parse(&packet.platform)
                        .unwrap_or(device::Platform::Unknown),
                    device_type: device::DeviceType::Unknown,
                    created_at: chrono::Utc::now(),
                    protocol_version: packet.protocol_version,
                    public_key_path: String::new(),
                    private_key_path: String::new(),
                };
                peer_manager.add_or_update_peer(identity, addr, packet.tcp_port).await;

                if packet.is_reply() {
                    // A reply to one of our beacons → RTT measured on our clock.
                    if let Some(t0) = packet.reply_to_sent_at_ms
                        && t0 > 0
                    {
                        let latency = now_unix_ms().saturating_sub(t0);
                        peer_manager.record_latency(&packet.device_id, latency).await;
                    }
                } else {
                    // A beacon → answer immediately so the sender can measure RTT.
                    let reply = beacon.reply_to(&packet, now_unix_ms());
                    let bytes = match serde_json::to_vec(&reply) {
                        Ok(b) => b,
                        Err(_) => continue,
                    };
                    if let Err(e) = socket.send_to(&bytes, addr).await {
                        tracing::warn!("[UDP] Reply to {addr} failed: {e}");
                    }
                }

                // Deterministic connection initiator — the higher-id side
                // establishes the TCP session, using the peer's advertised port.
                if packet.device_id > local_identity.id
                    && !session_manager.has_session(&packet.device_id).await
                {
                    let cm = connection_manager.clone();
                    let tcp_addr = SocketAddr::new(addr.ip(), packet.tcp_port);
                    tokio::spawn(async move {
                        if let Err(e) = cm.connect_to_peer(tcp_addr).await {
                            tracing::warn!("[UDP] Failed to connect to {}: {e}", packet.device_id);
                        }
                    });
                }
            }
            _ = shutdown.cancelled() => {
                tracing::info!("[UDP] Discovery shutdown");
                break;
            }
        }
    }
}

fn main() {
    // Build an explicit runtime so fatal errors can be reported cleanly
    // instead of panicking inside #[tokio::main].
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(err) => {
            eprintln!("UniKey daemon could not start a tokio runtime: {err}");
            std::process::exit(1);
        }
    };

    if let Err(err) = runtime.block_on(run()) {
        eprintln!("UniKey daemon failed: {err:#}");
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    // -------------------------------------------------------------------
    // 0. Bootstrap: paths, directories, logging, config
    // -------------------------------------------------------------------
    let paths = RuntimePaths::detect()?;
    paths.initialize()?;

    let config = Config::load_or_create(&paths.app_config_file())?;
    for warning in config.port_warnings().iter().chain(config.settings_warnings().iter()) {
        eprintln!("WARNING: {warning}");
    }

    logging::init(
        &paths,
        &config.logging.level,
        config.logging.file_enabled,
        config.logging.max_bytes,
        config.logging.max_backups,
    )?;

    let info = build_info::BuildInfo::capture();
    // Best effort: version metadata is diagnostic, never required.
    if let Err(err) = info.write_version_file(&paths.version_file()) {
        eprintln!("WARNING: could not write version.json: {err}");
    }

    tracing::info!(
        "UniKey daemon v{} ({} build, commit {}, platform {}, mode: {})",
        build_info::VERSION,
        build_info::PROFILE,
        build_info::COMMIT,
        build_info::platform(),
        paths.mode_name(),
    );

    // -------------------------------------------------------------------
    // 1. Device identity
    // -------------------------------------------------------------------
    let device_paths = device::paths::Paths::with_root(paths.config_dir().to_path_buf())?;
    let device_manager = DeviceManager::new_with_paths(device_paths)?;
    let mut local_identity = device_manager.load_or_create()?;

    // Optional display-name override from config.
    if let Some(name) = config.daemon.device_name.as_ref()
        && !name.trim().is_empty()
    {
        local_identity.name = name.trim().to_string();
    }

    // Load the long-term identity key from the persisted private key file.
    let local_identity_key = crypto::IdentityKey::load_private_file(&local_identity.private_key_path)?;

    tracing::info!(
        "Device: {} ({}) — state in {}",
        local_identity.name,
        local_identity.id,
        paths.config_dir().display()
    );

    // -------------------------------------------------------------------
    // 2. Core subsystems
    // -------------------------------------------------------------------
    let peer_manager = PeerManager::new();
    // A single event bus is shared by the transport (session events), every
    // service (they publish) and the HTTP API (which exposes it over SSE to
    // the desktop GUI). Created first so the session manager can attach to it.
    let event_bus = EventBus::new();
    let session_manager = SessionManager::new();
    session_manager.set_event_bus(event_bus.clone()).await;

    let (inbound_tx, inbound_rx) = mpsc::channel::<(uuid::Uuid, Packet)>(256);
    let mut inbound_rx = transport::motion::LatestMotionReceiver::new(inbound_rx);

    let connection_manager = ConnectionManager::new(
        local_identity.clone(),
        local_identity_key,
        session_manager.clone(),
        Some(inbound_tx),
    );

    // Backstop for sessions orphaned by a crashed task: drop any session that
    // has seen no activity in 10 minutes. The read loop normally tears down
    // connections first; this is defense in depth.
    session_manager
        .clone()
        .spawn_idle_sweeper(std::time::Duration::from_secs(60), std::time::Duration::from_secs(600));

    let shutdown_token = CancellationToken::new();

    // -------------------------------------------------------------------
    // 3. Service registration
    // -------------------------------------------------------------------
    // Pairing service is wrapped in Arc so the HTTP API can share it. Its
    // trust store lives in the same data directory as the device identity.
    let pairing_service = Arc::new(PairingService::with_store_root(Some(paths.config_dir()))?);
    let clipboard_service = Arc::new(ClipboardService::new());
    // Download location honours the user's setting and falls back to a
    // per-install downloads folder under the daemon's data directory.
    let download_dir = config
        .transfer
        .download_dir
        .clone()
        .unwrap_or_else(|| paths.config_dir().join("downloads"));
    let files_service = Arc::new(FileTransferService::with_download_dir(download_dir));
    let input_service = InputService::with_pairing(pairing_service.clone());
    let registry = ServiceRegistry::new();

    registry
        .register(Box::new(ArcPairingService(pairing_service.clone())))
        .await;
    if config.services.clipboard {
        registry.register(Box::new(ArcClipboardService(clipboard_service.clone()))).await;
    }
    if config.services.input {
        registry.register(Box::new(input_service.clone())).await;
    }
    if config.services.files {
        registry.register(Box::new(ArcFileTransferService(files_service.clone()))).await;
    }

    // Phase 2–6 placeholder implementations remain in the source tree for
    // planned work, but are deliberately not registered. Advertising no-op
    // placeholders as "running" made the API and desktop UI misleading.

    // -------------------------------------------------------------------
    // 4. Start services
    // -------------------------------------------------------------------
    let ctx = ServiceContext::with_events(
        local_identity.clone(),
        session_manager.clone(),
        shutdown_token.clone(),
        event_bus.clone(),
    );
    registry.start_all(ctx).await?;

    // -------------------------------------------------------------------
    // 5. Inbound packet routing — transport → service registry
    //    Authorization gate: an untrusted peer may only drive the pairing
    //    exchange. Anything else (clipboard, input, files) is rejected until
    //    the device is trusted. Transport-level packets (Ping/Pong/Hello/
    //    Disconnect) are handled by the connection layer and never reach here.
    // -------------------------------------------------------------------
    let router_registry = registry.clone();
    let router_pairing = pairing_service.clone();
    let shutdown_router = shutdown_token.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some((peer_id, packet)) = inbound_rx.recv() => {
                    let is_pairing = matches!(
                        packet.message_type,
                        MessageType::PairRequest | MessageType::PairResponse
                    );
                    if !is_pairing && !router_pairing.is_current_session_trusted(&peer_id).await {
                        tracing::warn!(
                            "[ROUTER] Dropping {:?} from untrusted peer {peer_id}",
                            packet.message_type
                        );
                        continue;
                    }
                    if let Err(e) = router_registry
                        .route_packet(&peer_id, packet.message_type, &packet.payload)
                        .await
                    {
                        tracing::warn!("[ROUTER] Failed to route packet from {peer_id}: {e}");
                    }
                }
                _ = shutdown_router.cancelled() => {
                    tracing::info!("[ROUTER] Shutting down");
                    break;
                }
            }
        }
    });

    // -------------------------------------------------------------------
    // 6. HTTP API server (axum) — Flutter IPC
    // -------------------------------------------------------------------
    let api_state = api::ApiState {
        pairing_service: pairing_service.clone(),
        session_manager: session_manager.clone(),
        peer_manager: peer_manager.clone(),
        local_identity: local_identity.clone(),
        connection_manager: connection_manager.clone(),
        registry: registry.clone(),
        build_info: info,
        config: Arc::new(tokio::sync::RwLock::new(config.clone())),
        paths: paths.clone(),
        event_bus: event_bus.clone(),
        clipboard_service: clipboard_service.clone(),
        files_service: files_service.clone(),
        input_service: input_service.clone(),
    };

    let api_router = api::build_router(api_state);
    let api_addr = format!("127.0.0.1:{}", config.network.api_port);
    let api_listener = tokio::net::TcpListener::bind(&api_addr)
        .await
        .with_context(|| format!("unable to bind required local API listener at {api_addr}"))?;
    let shutdown_api = shutdown_token.clone();
    tokio::spawn(async move {
        tracing::info!("[API] Server listening on {api_addr}");
        axum::serve(api_listener, api_router)
            .with_graceful_shutdown(async move { shutdown_api.cancelled().await })
            .await
            .unwrap_or_else(|e| tracing::error!("[API] Server error: {e}"));
    });

    // -------------------------------------------------------------------
    // 7. Stale peer cleanup (background)
    // -------------------------------------------------------------------
    let pm_cleanup = peer_manager.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(5));
        let stale_after = Duration::from_secs(15);
        let prune_after = Duration::from_secs(60);
        loop {
            interval.tick().await;
            pm_cleanup.cleanup_stale_peers(stale_after).await;
            // Remove peers that have been offline long enough to be "gone".
            pm_cleanup.prune_offline(prune_after).await;
        }
    });

    // -------------------------------------------------------------------
    // 8. TCP transport — listen for incoming connections
    // -------------------------------------------------------------------
    let cm_tcp = connection_manager.clone();
    let shutdown_tcp = shutdown_token.clone();
    let bind_addr = format!(
        "{}:{}",
        config.network.tcp_bind, config.network.tcp_port
    );
    let tcp_listener = TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("unable to bind required TCP transport listener at {bind_addr}"))?;
    tokio::spawn(async move {
        tracing::info!("[TCP] Listening on {bind_addr}");

        loop {
            tokio::select! {
                result = tcp_listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            tracing::debug!("[TCP] Incoming from {addr}");
                            let cm = cm_tcp.clone();
                            tokio::spawn(async move {
                                if let Err(e) = cm.handle_incoming(stream).await {
                                    tracing::warn!("[TCP] Connection error from {addr}: {e}");
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("[TCP] Accept error: {e}");
                            break;
                        }
                    }
                }
                _ = shutdown_tcp.cancelled() => {
                    tracing::info!("[TCP] Shutting down");
                    break;
                }
            }
        }
    });

    // -------------------------------------------------------------------
    // 9. UDP discovery — broadcast beacons on the LAN and listen for peers
    // -------------------------------------------------------------------
    // Non-fatal: if the discovery socket can't bind (e.g. a second instance on
    // the same host), the daemon still serves the HTTP API and TCP transport —
    // it just runs without LAN discovery.
    let udp_future: Pin<Box<dyn Future<Output = ()> + Send>> =
        match build_udp_socket(&config.network.discovery_bind, config.network.discovery_port) {
            Ok(socket) => Box::pin(run_udp_discovery(
                Arc::new(socket),
                local_identity.clone(),
                config.network.discovery_port,
                config.network.tcp_port,
                peer_manager.clone(),
                connection_manager.clone(),
                session_manager.clone(),
                shutdown_token.clone(),
            )),
            Err(e) => {
                tracing::warn!(
                    "[UDP] Discovery disabled — could not bind {}:{}: {e}",
                    config.network.discovery_bind,
                    config.network.discovery_port
                );
                Box::pin(std::future::pending())
            }
        };

    // -------------------------------------------------------------------
    // 10. Wait for Ctrl+C
    // -------------------------------------------------------------------
    tokio::select! {
        _ = udp_future => {}
        _ = signal::ctrl_c() => {
            tracing::info!("Shutting down...");
            shutdown_token.cancel();
        }
    }

    registry.stop_all().await?;
    logging::flush_before_exit();
    tracing::info!("[DAEMON] Goodbye");
    Ok(())
}
