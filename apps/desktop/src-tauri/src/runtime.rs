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
        Some(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/unikey-daemon.bin"
        )))
    }
    #[cfg(not(unikey_embed_daemon))]
    {
        None
    }
}

/// Private, per-user application directory where extracted binaries live.
///
/// Installed application folders are not reliably writable (notably
/// `Program Files` on Windows and `/Applications` on macOS), so an embedded
/// daemon must never be extracted beside the GUI executable.
fn runtime_dir() -> Result<PathBuf, String> {
    dirs::data_local_dir()
        .map(|base| base.join("UniKey").join("runtime"))
        .ok_or_else(|| "cannot determine the per-user application data directory".to_string())
}

/// Give each embedded daemon payload a content-addressed filename. Windows
/// does not allow replacing a running executable, so versioned filenames make
/// upgrades reliable even if a previous app instance crashed and left its
/// daemon alive.
fn embedded_daemon_name(bytes: &[u8]) -> String {
    // FNV-1a is small, stable across Rust releases, and sufficient here: the
    // fingerprint is only a deterministic filename, not a security boundary.
    let fingerprint = bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    });
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    format!("unikey-daemon-{fingerprint:016x}{suffix}")
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
    let dir = runtime_dir()?;
    fs::create_dir_all(&dir)
        .map_err(|e| format!("cannot create runtime dir {}: {e}", dir.display()))?;

    let embedded_name = embedded_daemon_name(bytes);
    let target = dir.join(&embedded_name);
    let refreshed = !target.exists();

    if refreshed {
        let tmp = dir.join(format!("{embedded_name}.new"));
        fs::write(&tmp, bytes)
            .map_err(|e| format!("cannot write daemon {}: {e}", tmp.display()))?;
        set_executable(&tmp)?;

        // The target is content-addressed, so it is never a running older
        // executable. Rename keeps a partial extraction from being launched.
        if let Err(e) = fs::rename(&tmp, &target) {
            let _ = fs::remove_file(&tmp); // clean up the partial write
            return Err(format!(
                "cannot install embedded daemon {}: {e}",
                target.display()
            ));
        }
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

#[cfg(test)]
mod tests {
    use super::{embedded_daemon_name, runtime_dir};

    #[test]
    fn embedded_names_are_stable_and_content_addressed() {
        assert_eq!(embedded_daemon_name(b"same"), embedded_daemon_name(b"same"));
        assert_ne!(embedded_daemon_name(b"old"), embedded_daemon_name(b"new"));
    }

    #[test]
    fn embedded_runtime_uses_per_user_data() {
        let runtime = runtime_dir().expect("per-user data directory");
        let executable = std::env::current_exe().expect("current executable");
        assert!(!runtime.starts_with(executable.parent().expect("executable parent")));
        assert!(runtime.ends_with("UniKey/runtime"));
    }
}
