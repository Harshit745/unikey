//! UniKey desktop shell — Tauri v2.
//!
//! This crate is a thin native shell around the UniKey daemon:
//!
//!   1. Resolve the daemon binary (embedded in this exe, or side-by-side in a
//!      portable folder, or the developer's `target/release/daemon`).
//!   2. Extract/refresh the embedded runtime into a private application dir.
//!   3. Spawn the daemon and wait for the local API to report healthy.
//!   4. Present the React UI, which talks to the daemon over HTTP only.
//!
//! All LAN/transport logic lives in the daemon — this shell never touches
//! sockets beyond the localhost health check and the localhost SSE stream
//! used to drive desktop notifications.

mod daemon;
mod notify;
mod runtime;

use std::sync::Mutex;

use daemon::DaemonHandle;
use tauri::{Manager, State};

/// Shared launcher state, handed to commands as `State<'_, LauncherState>`.
struct LauncherState {
    /// Serializes health-check/restart operations so two UI calls cannot spawn
    /// competing daemon children.
    launch: tokio::sync::Mutex<()>,
    /// Resolved runtime layout + the last daemon handle we started.
    runtime: Mutex<Option<runtime::ExtractedRuntime>>,
    handle: Mutex<Option<DaemonHandle>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let runtime = match runtime::resolve_runtime() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[unikey] runtime resolution failed: {e}");
            runtime::ExtractedRuntime::dev_fallback()
        }
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Focus the existing window when a second instance is launched.
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_notification::init())
        .manage(LauncherState {
            launch: tokio::sync::Mutex::new(()),
            runtime: Mutex::new(Some(runtime)),
            handle: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            daemon_start,
            daemon_status,
            runtime_info,
            edge_capture_start,
            edge_capture_stop,
        ])
        .setup(|app| {
            setup_tray(app)?;
            // Forward daemon events (sessions, transfers, pairing) to native
            // desktop notifications. Best effort — never blocks startup.
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                notify::watch_daemon_events(app_handle).await;
            });
            setup_installer_smoke_exit(app);
            Ok(())
        })
        .on_window_event(|window, event| {
            // Keep UniKey and its managed daemon available in the tray. The
            // explicit tray Quit action performs the real shutdown.
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building the UniKey desktop shell")
        .run(|app, event| {
            if matches!(
                event,
                tauri::RunEvent::Exit | tauri::RunEvent::ExitRequested { .. }
            ) {
                shutdown_owned_daemon(app.state::<LauncherState>().inner());
            }
        });
}

/// CI launches the installed application with `--installer-smoke-test`.
/// The normal React startup still extracts and starts the real daemon; this
/// watchdog exits successfully only after the localhost health endpoint is
/// live. That exercises the packaged binary and graceful child shutdown
/// without adding a fake transport path to production behavior.
fn setup_installer_smoke_exit(app: &tauri::App) {
    if !std::env::args().any(|arg| arg == "--installer-smoke-test") {
        return;
    }

    let app_handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while std::time::Instant::now() < deadline {
            let health = daemon::health_json().await;
            if matches!(
                health.get("status").and_then(|value| value.as_str()),
                Some("ok" | "running")
            ) {
                // Leave a short observation window so the Windows smoke
                // script can query /health, /version, and /services while the
                // installed process and its real daemon are both alive.
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                app_handle.exit(0);
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
        app_handle.exit(2);
    });
}

fn shutdown_owned_daemon(state: &LauncherState) {
    if let Some(mut handle) = state.handle.lock().unwrap().take() {
        handle.shutdown();
    }
}

// ── System tray ────────────────────────────────────────────────────────

/// Keep the tray icon alive for the app's lifetime. The `Drop` impl of
/// `TrayIcon` would otherwise remove it as soon as `setup` returns — the
/// field is intentionally never read, only held.
#[allow(dead_code)]
struct TrayHolder(tauri::tray::TrayIcon);

/// Build the menu bar / dock tray: show the window, or quit the app.
fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder};
    use tauri::tray::{TrayIconBuilder, TrayIconEvent};

    let show = MenuItemBuilder::with_id("show", "Show UniKey").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
    let menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;

    // Without a bundled icon there is nothing to render — skip the tray.
    let Some(icon) = app.default_window_icon().cloned() else {
        return Ok(());
    };

    let tray = TrayIconBuilder::with_id("unikey-tray")
        .icon(icon)
        .tooltip("UniKey — one keyboard, one clipboard, one workspace")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "quit" => {
                // Closing the window hides it to the tray; quit is explicit.
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(event, TrayIconEvent::DoubleClick { .. }) {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    app.manage(TrayHolder(tray));
    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum ScreenEdge {
    Left,
    Right,
}

/// Create a narrow native drop target on a physical screen edge. Files can be
/// dragged here from Finder/Explorer without first opening the UniKey window.
/// The edge page forwards the dropped paths to the daemon for the selected,
/// already-paired peer.
#[tauri::command]
async fn edge_capture_start(
    app: tauri::AppHandle,
    peer_id: String,
    edge: ScreenEdge,
) -> Result<(), String> {
    uuid::Uuid::parse_str(&peer_id).map_err(|_| "invalid peer id".to_string())?;

    if let Some(existing) = app.get_webview_window("edge-drop") {
        existing.close().map_err(|e| e.to_string())?;
    }

    let monitor = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no primary monitor available".to_string())?;
    let scale = monitor.scale_factor();
    let size = monitor.size().to_logical::<f64>(scale);
    let origin = monitor.position().to_logical::<f64>(scale);
    let strip_width = 18.0;
    let x = match edge {
        ScreenEdge::Left => origin.x,
        ScreenEdge::Right => origin.x + size.width - strip_width,
    };
    let url = format!("index.html#/edge-drop?peer={peer_id}");

    tauri::WebviewWindowBuilder::new(&app, "edge-drop", tauri::WebviewUrl::App(url.into()))
        .title("UniKey edge handoff")
        .position(x, origin.y)
        .inner_size(strip_width, size.height)
        .decorations(false)
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .closable(false)
        .focused(false)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .skip_taskbar(true)
        .shadow(false)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn edge_capture_stop(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("edge-drop") {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── Tauri commands (called from the React UI via `invoke`) ─────────────

/// Start the daemon (extracting the runtime first if needed) and wait for the
/// local API to report healthy. Safe to call repeatedly — only one daemon is
/// ever spawned per GUI session.
#[tauri::command]
async fn daemon_start(state: State<'_, LauncherState>) -> Result<serde_json::Value, String> {
    let _launch_guard = state.launch.lock().await;

    let health = daemon::health_json().await;
    if health.get("status").and_then(|value| value.as_str()) == Some("ok")
        || health.get("status").and_then(|value| value.as_str()) == Some("running")
    {
        return Ok(health);
    }

    // An owned daemon stopped responding or exited. Reap it before starting
    // a replacement so retries recover from crashes and stale port state.
    if let Some(mut existing) = state.handle.lock().unwrap().take() {
        if existing.is_running() {
            existing.shutdown();
        }
    }

    let runtime = state
        .runtime
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "runtime not resolved".to_string())?;

    let handle = match daemon::spawn_daemon(&runtime).await {
        Ok(handle) => handle,
        Err(error) => return Err(error.to_string()),
    };
    *state.handle.lock().unwrap() = Some(handle);

    Ok(daemon::health_json().await)
}

/// Report whether the daemon is reachable via its health endpoint.
#[tauri::command]
async fn daemon_status(state: State<'_, LauncherState>) -> Result<serde_json::Value, String> {
    let _ = state; // status is derived from the HTTP API, not local state
    Ok(daemon::health_json().await)
}

/// Describe the resolved runtime layout (embedded vs portable vs dev).
#[tauri::command]
fn runtime_info(state: State<'_, LauncherState>) -> Result<serde_json::Value, String> {
    let runtime = state.runtime.lock().unwrap().clone();
    let handle = state.handle.lock().unwrap();
    match runtime {
        Some(r) => Ok(serde_json::json!({
            "origin": r.origin.as_str(),
            "runtime_dir": r.dir.display().to_string(),
            "daemon_path": r.daemon_path.display().to_string(),
            "daemon_pid": handle.as_ref().and_then(|h| h.pid()),
        })),
        None => Err("runtime not resolved".to_string()),
    }
}
