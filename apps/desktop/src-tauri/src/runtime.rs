//! Runtime layout resolution for the UniKey daemon.
//!
//! The daemon binary can come from three places, in priority order:
//!
//!   1. **Embedded** — packaged inside the GUI executable (Windows portable
//!      build). Extracted to a private application data dir on first launch.
//!   2. **Portable** — a `daemon` (or `daemon.exe`) sitting next to the GUI
//!      binary, detected via a `unikey.portable` marker file.
//!   3. **Dev** — the developer's `target/release/daemon` built from this
//!      workspace.
//!
//! This module resolves which layout is active and — for the embedded case —
//! extracts the runtime and replaces it atomically when the embedded version
//! is newer.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::daemon;

/// Marker file that switches a folder into "portable" mode.
const PORTABLE_MARKER: &str = "unikey.portable";

/// Where the embedded daemon is stored inside the extracted runtime dir.
const EMBEDDED_DAEMON_NAME: &str = daemon::DAEMON_EXE_NAME;

/// Which runtime layout is in effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RuntimeOrigin {
    Embedded,
    Portable,
    Dev,
}

impl RuntimeOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeOrigin::Embedded => "embedded",
            RuntimeOrigin::Portable => "portable",
            RuntimeOrigin::Dev => "dev",
        }
    }
}

/// A fully-resolved runtime: where the daemon lives and where its data lives.
#[derive(Debug, Clone, Serialize)]
pub struct ExtractedRuntime {
    pub origin: RuntimeOrigin,
    /// Directory that holds the daemon binary (and its data).
    pub dir: PathBuf,
    /// Path to the daemon executable.
    pub daemon_path: PathBuf,
    /// True when a newer embedded runtime was swapped in this launch.
    pub refreshed: bool,
}

impl ExtractedRuntime {
    /// Fallback used when resolution fails: point at the workspace's release
    /// build so the app still boots in development.
    pub fn dev_fallback() -> Self {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("target")
            .join("release");
        Self {
            origin: RuntimeOrigin::Dev,
            daemon_path: dir.join(daemon::DAEMON_EXE_NAME),
            dir,
            refreshed: false,
        }
    }
}

/// Embedded daemon bytes, baked in by the packaging build when present.
///
/// The `unikey_embed_daemon` cfg is emitted by build.rs only when a daemon
/// binary exists at the expected path, so plain `cargo build` (development)
/// compiles without one.
fn embedded_daemon_bytes() -> Option<&'static [u8]> {
    #[cfg(unikey_embed_daemon)]
    {
        Some(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/unikey-daemon.bin")))
    }
    #[cfg(not(unikey_embed_daemon))]
    {
        None
    }
}

/// Private application directory where the extracted daemon lives.
fn runtime_dir() -> PathBuf {
    let base = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("runtime")
}

/// Resolve the active runtime layout, extracting/refreshing an embedded
/// runtime when one is present.
pub fn resolve_runtime() -> Result<ExtractedRuntime, String> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
        .ok_or_else(|| "cannot determine executable directory".to_string())?;

    // 1. Portable layout wins if the marker file sits next to the GUI.
    if exe_dir.join(PORTABLE_MARKER).exists() {
        let daemon_path = exe_dir.join(daemon::DAEMON_EXE_NAME);
        if daemon_path.exists() {
            return Ok(ExtractedRuntime {
                origin: RuntimeOrigin::Portable,
                dir: exe_dir,
                daemon_path,
                refreshed: false,
            });
        }
    }

    // 2. Embedded daemon: extract into the private runtime dir.
    if let Some(bytes) = embedded_daemon_bytes() {
        if !bytes.is_empty() {
            return extract_embedded(bytes);
        }
    }

    // 3. Development fallback.
    Ok(ExtractedRuntime::dev_fallback())
}

/// Write the embedded daemon to the runtime dir, replacing it atomically when
/// the on-disk copy is stale.
fn extract_embedded(bytes: &'static [u8]) -> Result<ExtractedRuntime, String> {
    let dir = runtime_dir();
    fs::create_dir_all(&dir)
        .map_err(|e| format!("cannot create runtime dir {}: {e}", dir.display()))?;

    let target = dir.join(EMBEDDED_DAEMON_NAME);
    let mut refreshed = false;

    // Compare sizes as a cheap staleness check — a differing size (or a
    // missing file) means we should replace.
    let stale = match fs::metadata(&target) {
        Ok(m) => m.len() != bytes.len() as u64,
        Err(_) => true,
    };

    if stale {
        let tmp = dir.join(format!("{EMBEDDED_DAEMON_NAME}.new"));
        fs::write(&tmp, bytes)
            .map_err(|e| format!("cannot write daemon {}: {e}", tmp.display()))?;
        set_executable(&tmp)?;

        // Atomic rename replaces any running copy's file on disk. On Windows
        // this can fail if the old process still holds the file open — that
        // case is handled by the caller (crash recovery).
        if let Err(e) = fs::rename(&tmp, &target) {
            let _ = fs::remove_file(&tmp); // clean up the partial write
            return Err(format!(
                "cannot replace daemon {}: {e} (is an old daemon still running?)",
                target.display()
            ));
        }
        refreshed = true;
    }

    Ok(ExtractedRuntime {
        origin: RuntimeOrigin::Embedded,
        dir,
        daemon_path: target,
        refreshed,
    })
}

/// Best-effort chmod +x. Errors are non-fatal on Windows.
fn set_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)
            .map_err(|e| format!("cannot stat {}: {e}", path.display()))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)
            .map_err(|e| format!("cannot chmod {}: {e}", path.display()))?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}
