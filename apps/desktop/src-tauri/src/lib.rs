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
    /// Tracks whether we already spawned the daemon this session.
    spawned: Mutex<bool>,
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
            spawned: Mutex::new(false),
            runtime: Mutex::new(Some(runtime)),
            handle: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            daemon_start,
            daemon_status,
            runtime_info,
        ])
        .setup(|app| {
            setup_tray(app)?;
            // Forward daemon events (sessions, transfers, pairing) to native
            // desktop notifications. Best effort — never blocks startup.
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                notify::watch_daemon_events(app_handle).await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running the UniKey desktop shell");
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

// ── Tauri commands (called from the React UI via `invoke`) ─────────────

/// Start the daemon (extracting the runtime first if needed) and wait for the
/// local API to report healthy. Safe to call repeatedly — only one daemon is
/// ever spawned per GUI session.
#[tauri::command]
async fn daemon_start(state: State<'_, LauncherState>) -> Result<serde_json::Value, String> {
    // Check whether we already spawned the daemon this session.
    let already_spawned = {
        let mut guard = state.spawned.lock().unwrap();
        if *guard {
            true
        } else {
            *guard = true;
            false
        }
    };

    if already_spawned {
        return daemon_status(state).await;
    }

    let runtime = state
        .runtime
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "runtime not resolved".to_string())?;

    let handle = match daemon::spawn_daemon(&runtime).await {
        Ok(handle) => handle,
        Err(error) => {
            // A missing/stale binary or a transient port conflict must not
            // permanently disable the Retry action for this app session.
            *state.spawned.lock().unwrap() = false;
            return Err(error.to_string());
        }
    };
    *state.handle.lock().unwrap() = Some(handle);

    Ok(daemon_status(state).await?)
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
    let handle = state.handle.lock().unwrap().clone();
    match runtime {
        Some(r) => Ok(serde_json::json!({
            "origin": r.origin.as_str(),
            "runtime_dir": r.dir.display().to_string(),
            "daemon_path": r.daemon_path.display().to_string(),
            "daemon_pid": handle.map(|h| h.pid()),
        })),
        None => Err("runtime not resolved".to_string()),
    }
}
