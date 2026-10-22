//! Daemon process management for the UniKey desktop shell.
//!
//! This module owns the daemon subprocess: spawning it, health-checking it,
//! and (best-effort) shutting it down when the GUI exits.

use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::runtime::ExtractedRuntime;

/// Daemon binary name (platform-dependent).
#[cfg(windows)]
pub const DAEMON_EXE_NAME: &str = "daemon.exe";
#[cfg(not(windows))]
pub const DAEMON_EXE_NAME: &str = "daemon";

/// Local API base URL — matches the daemon's bind address.
pub const API_BASE: &str = "http://127.0.0.1:45895";

/// How long to wait for the daemon to report healthy after spawning.
const HEALTH_TIMEOUT: Duration = Duration::from_secs(15);
/// Poll interval while waiting for health.
const HEALTH_POLL: Duration = Duration::from_millis(250);

/// A live handle to the daemon subprocess.
#[derive(Debug)]
pub struct DaemonHandle {
    pid: Option<u32>,
    child: Option<Child>,
}

impl DaemonHandle {
    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    /// Whether the child process owned by this app is still alive.
    pub fn is_running(&mut self) -> bool {
        match self.child.as_mut() {
            Some(child) => matches!(child.try_wait(), Ok(None)),
            None => false,
        }
    }

    /// Stop only the daemon spawned by this app. A daemon that was already
    /// running before launch is deliberately never terminated here.
    pub fn shutdown(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for DaemonHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Spawn the daemon and wait until its `/health` returns HTTP 200.
///
/// If a daemon is *already* reachable (e.g. launched by an earlier instance),
/// this returns immediately with no new process.
pub async fn spawn_daemon(runtime: &ExtractedRuntime) -> Result<DaemonHandle, SpawnError> {
    // Already healthy? Nothing to do.
    if health_json().await.status_ok() {
        return Ok(DaemonHandle {
            pid: None,
            child: None,
        });
    }

    let exe = &runtime.daemon_path;
    if !exe.exists() {
        return Err(SpawnError::MissingBinary(exe.clone()));
    }

    // The GUI owns this child. Closing the window hides UniKey to the tray;
    // choosing Quit terminates the daemon through `DaemonHandle::shutdown`.
    let mut cmd = std::process::Command::new(exe);
    cmd.current_dir(&runtime.dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW: the daemon is a background child and must never
        // flash a terminal window on launch.
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| SpawnError::Launch(exe.clone(), e))?;
    let pid = child.id();

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Err(SpawnError::Exited(status)),
            Ok(None) => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(SpawnError::Inspect(error));
            }
        }
        if health_json().await.status_ok() {
            return Ok(DaemonHandle {
                pid: Some(pid),
                child: Some(child),
            });
        }
        if started.elapsed() >= HEALTH_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err(SpawnError::Timeout(HEALTH_TIMEOUT));
        }
        tokio::time::sleep(HEALTH_POLL).await;
    }
}

/// Fetch the daemon `/health` payload. Returns `{"status":"down"}` on any
/// error so the UI always gets a well-formed answer.
pub async fn health_json() -> serde_json::Value {
    let url = format!("{API_BASE}/health");
    match reqwest(&url, Duration::from_millis(1500)).await {
        Ok(body) => serde_json::from_str(&body)
            .map(normalize_health_body)
            .unwrap_or_else(|_| json_down()),
        Err(_) => json_down(),
    }
}

/// Convert the daemon's standard API envelope into the health payload used by
/// the native launcher. Keeping one normalized shape prevents the launcher and
/// React client from silently disagreeing about liveness.
fn normalize_health_body(value: serde_json::Value) -> serde_json::Value {
    if value.get("success").and_then(|v| v.as_bool()) == Some(true) {
        value.get("data").cloned().unwrap_or_else(json_down)
    } else {
        value
    }
}

/// Small wrapper for ergonomic status checks on the health payload.
trait HealthJsonExt {
    fn status_ok(&self) -> bool;
}

impl HealthJsonExt for serde_json::Value {
    fn status_ok(&self) -> bool {
        matches!(
            self.get("status").and_then(|v| v.as_str()),
            Some("running" | "ok")
        )
    }
}

fn json_down() -> serde_json::Value {
    serde_json::json!({ "status": "down", "endpoint": API_BASE })
}

/// Minimal HTTP GET on localhost using tokio's async TCP.
async fn reqwest(url: &str, timeout: Duration) -> Result<String, String> {
    let Some(rest) = url.split("://").nth(1).map(str::to_string) else {
        return Err("bad url".into());
    };
    let (host_port, path) = match rest.split_once('/') {
        Some((h, p)) => (h.to_string(), format!("/{p}")),
        None => (rest, "/".to_string()),
    };

    let fut = async {
        let mut stream = TcpStream::connect(&host_port)
            .await
            .map_err(|e| e.to_string())?;
        stream.set_nodelay(true).map_err(|e| e.to_string())?;
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\n\r\n");
        stream
            .write_all(request.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        stream.flush().await.map_err(|e| e.to_string())?;
        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .await
            .map_err(|e| e.to_string())?;
        Ok::<String, String>(
            String::from_utf8_lossy(&response)
                .split("\r\n\r\n")
                .nth(1)
                .unwrap_or("")
                .to_string(),
        )
    };

    tokio::time::timeout(timeout, fut)
        .await
        .map_err(|_| "request timeout".to_string())?
}

/// Errors from launching the daemon.
#[derive(Debug, thiserror::Error)]
pub enum SpawnError {
    #[error("daemon binary not found at {0}")]
    MissingBinary(PathBuf),
    #[error("failed to launch daemon at {0}: {1}")]
    Launch(PathBuf, std::io::Error),
    #[error("failed to inspect daemon process: {0}")]
    Inspect(std::io::Error),
    #[error("daemon exited before reporting healthy ({0})")]
    Exited(std::process::ExitStatus),
    #[error("daemon did not report healthy within {0:?}")]
    Timeout(Duration),
}

impl SpawnError {
    #[allow(dead_code)]
    pub fn message(&self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_health_body, HealthJsonExt};

    #[test]
    fn normalizes_daemon_health_envelope() {
        let normalized = normalize_health_body(serde_json::json!({
            "success": true,
            "data": { "status": "running", "version": "0.1.0" },
            "error": null
        }));

        assert_eq!(normalized["status"], "running");
        assert!(normalized.status_ok());
    }

    #[test]
    fn rejected_or_down_health_is_not_healthy() {
        let rejected = normalize_health_body(serde_json::json!({
            "success": false,
            "data": null,
            "error": "unavailable"
        }));
        assert!(!rejected.status_ok());
        assert!(!serde_json::json!({ "status": "down" }).status_ok());
    }
}
