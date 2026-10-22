//! Desktop notifications driven by the daemon's SSE event stream.
//!
//! Opens `GET /api/v1/events` on the localhost API and turns interesting
//! events (a peer connected, a file arrived, pairing completed) into native
//! notifications. The stream is long-lived; on any disconnect we wait and
//! reconnect, so notifications resume automatically after the daemon restarts.

use std::time::Duration;

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;

/// Daemon API base (kept in sync with `daemon::API_BASE`).
const API_HOST: &str = "127.0.0.1:45895";
/// Backoff between reconnect attempts.
const RETRY_DELAY: Duration = Duration::from_secs(3);

/// Keep watching the daemon's event stream for the lifetime of the shell.
/// Never returns — the caller spawns this on a background task.
pub async fn watch_daemon_events(app: AppHandle) {
    loop {
        if let Err(e) = drain_stream(&app).await {
            eprintln!("[unikey] notification stream error: {e}");
        }
        // Back off before reconnecting (stream died or daemon restarted).
        tokio::time::sleep(RETRY_DELAY).await;
    }
}

/// Connect and read events until the stream closes.
async fn drain_stream(app: &AppHandle) -> Result<(), String> {
    let mut stream = TcpStream::connect(API_HOST).await.map_err(|e| e.to_string())?;
    stream.set_nodelay(true).map_err(|e| e.to_string())?;

    let request = format!(
        "GET /api/v1/events HTTP/1.1\r\n\
         Host: {API_HOST}\r\n\
         Accept: text/event-stream\r\n\
         Cache-Control: no-cache\r\n\
         Connection: close\r\n\
         \r\n"
    );
    use tokio::io::AsyncWriteExt;
    stream.write_all(request.as_bytes()).await.map_err(|e| e.to_string())?;
    stream.flush().await.map_err(|e| e.to_string())?;

    let mut lines = BufReader::new(stream).lines();
    while let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? {
        // SSE data frames look like: `data: { "type": "...", ... }`
        if let Some(payload) = line.strip_prefix("data:") {
            let payload = payload.trim();
            if !payload.is_empty() {
                handle_frame(app, payload);
            }
        }
    }
    Err("stream closed".into())
}

/// Map one SSE frame to a (title, body) notification pair, or `None` for
/// routine events (clipboard sync, progress) and malformed frames.
fn notification_for(payload: &str) -> Option<(String, String)> {
    let value: serde_json::Value = serde_json::from_str(payload).ok()?;
    let event_type = value.get("type")?.as_str()?;
    let str_field = |k: &str| value.get(k).and_then(|v| v.as_str()).unwrap_or("");

    let (title, body) = match event_type {
        "session_connected" => (
            "Device connected",
            format!("{} is now linked", str_field("peer_name")),
        ),
        "session_disconnected" => (
            "Device disconnected",
            format!("{} went offline", str_field("peer_name")),
        ),
        "transfer_completed" => (
            "File transfer complete",
            format!("{} was transferred", str_field("filename")),
        ),
        "transfer_failed" => (
            "File transfer failed",
            str_field("error").to_string(),
        ),
        "pairing_completed" => (
            "Device paired",
            format!("{} was added to trusted devices", str_field("peer_name")),
        ),
        "device_removed" => ("Device removed", str_field("peer_name").to_string()),
        _ => return None, // routine events (clipboard sync, progress) are silent
    };
    Some((title.to_string(), body))
}

/// Parse one SSE frame and raise a notification if it is worth the user's
/// attention. Best effort — malformed frames are ignored.
fn handle_frame(app: &AppHandle, payload: &str) {
    let Some((title, body)) = notification_for(payload) else {
        return;
    };

    // Permission prompt + delivery is handled by the OS; never block on it.
    let _ = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .ok();
}

#[cfg(test)]
mod tests {
    use super::notification_for;

    fn payload(kind: &str) -> String {
        format!(r#"{{"type":"{kind}","peer_name":"imac","filename":"x.pdf","error":"boom"}}"#)
    }

    #[test]
    fn interesting_events_produce_notifications() {
        for (kind, expected_title) in [
            ("session_connected", "Device connected"),
            ("session_disconnected", "Device disconnected"),
            ("transfer_completed", "File transfer complete"),
            ("transfer_failed", "File transfer failed"),
            ("pairing_completed", "Device paired"),
            ("device_removed", "Device removed"),
        ] {
            let (title, body) = notification_for(&payload(kind)).expect("notification");
            assert_eq!(title, expected_title);
            assert!(!body.is_empty(), "body must be non-empty for {kind}");
        }
    }

    #[test]
    fn routine_events_are_silent() {
        assert!(notification_for(&payload("clipboard_synced")).is_none());
        assert!(notification_for(&payload("transfer_progress")).is_none());
        assert!(notification_for(&payload("transfer_started")).is_none());
        assert!(notification_for(&payload("service_status")).is_none());
    }

    #[test]
    fn malformed_and_unknown_frames_are_ignored() {
        assert!(notification_for("not json").is_none());
        assert!(notification_for("").is_none());
        assert!(notification_for(r#"{"no_type":true}"#).is_none());
        assert!(notification_for(&payload("totally_unknown_event")).is_none());
    }

    #[test]
    fn missing_fields_fall_back_to_empty_not_broken() {
        // A frame with a type but no peer_name must still notify safely.
        let (title, body) = notification_for(r#"{"type":"session_connected"}"#).unwrap();
        assert_eq!(title, "Device connected");
        assert!(body.contains("is now linked"));
    }
}
