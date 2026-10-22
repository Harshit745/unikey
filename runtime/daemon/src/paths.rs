//! Runtime path resolution for the daemon.
//!
//! UniKey supports two runtime layouts:
//!
//! ## Portable (release folder)
//!
//! When the file `unikey.portable` sits next to the daemon executable, all
//! mutable state (config, keys, device identity, logs) is kept **inside the
//! executable's folder**:
//!
//! ```text
//! UniKey/
//! ├── unikey.portable      <- marker file shipped by the build scripts
//! ├── daemon               <- this executable
//! ├── assets/
//! ├── config/              <- device.json, trusted.json, unikey.toml, keys/
//! ├── logs/                <- latest.log, panic.log
//! └── version.json
//! ```
//!
//! Copying that folder to another machine and running `daemon` "just works":
//! no installers, no shell, no per-user data.
//!
//! ## Standard (development / installed)
//!
//! Without the marker, the daemon uses the OS convention. All state lives
//! under the user's per-user config directory (`~/Library/Application Support`
//! on macOS, `%APPDATA%` on Windows, `~/.config` on Linux) under `UniKey/`,
//! preserving the layout used before this module existed.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// File whose presence next to the executable switches on portable mode.
pub const PORTABLE_MARKER: &str = "unikey.portable";

/// Which layout the daemon is running in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathMode {
    /// State is kept next to the executable (release folder).
    Portable,
    /// State is kept in the OS per-user config directory.
    Standard,
}

/// All runtime-visible locations, derived once at startup.
///
/// Every field is derived from the executable path (portable) or the OS
/// config directory (standard) — nothing here is ever hardcoded.
#[derive(Debug, Clone)]
pub struct RuntimePaths {
    mode: PathMode,
    /// Top-level folder that contains `config/`, `logs/` and `assets/`
    /// (portable) — or the OS app directory itself (standard).
    root: PathBuf,
    /// Folder holding mutable application data: `device.json`,
    /// `trusted.json`, `unikey.toml` and `keys/`.
    config_dir: PathBuf,
    /// Folder holding `latest.log` and `panic.log`.
    logs_dir: PathBuf,
    /// Folder holding read-only assets (icons, fonts, themes, locale).
    assets_dir: PathBuf,
}

impl RuntimePaths {
    /// Detect the runtime layout from the current executable's location.
    pub fn detect() -> Result<Self> {
        let exe = std::env::current_exe()
            .context("unable to locate the running executable")?;
        let exe_dir = exe
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        Self::detect_from(&exe_dir)
    }

    /// Detect the layout given the executable's directory (injectable for tests).
    pub fn detect_from(exe_dir: &Path) -> Result<Self> {
        let (mode, root) = if exe_dir.join(PORTABLE_MARKER).is_file() {
            (PathMode::Portable, exe_dir.to_path_buf())
        } else {
            let base = dirs::config_dir().context("unable to locate the OS config directory")?;
            (PathMode::Standard, base.join("UniKey"))
        };

        let (config_dir, logs_dir, assets_dir) = match mode {
            PathMode::Portable => (
                root.join("config"),
                root.join("logs"),
                root.join("assets"),
            ),
            PathMode::Standard => (
                root.clone(),
                root.join("logs"),
                root.join("assets"),
            ),
        };

        Ok(Self {
            mode,
            root,
            config_dir,
            logs_dir,
            assets_dir,
        })
    }

    // ── Mode --------------------------------------------------------------

    // The accessors below are part of the daemon's runtime path API. They are
    // not all consumed by today's binary (some serve the pairing wiring and
    // the future auto-updater), so allow them to be temporarily unused.

    /// Which layout is active.
    #[allow(dead_code)]
    pub fn mode(&self) -> PathMode {
        self.mode
    }

    /// Human-readable layout name, for logs and diagnostics.
    pub fn mode_name(&self) -> &'static str {
        match self.mode {
            PathMode::Portable => "portable",
            PathMode::Standard => "standard",
        }
    }

    // ── Directories -------------------------------------------------------

    /// Top-level folder holding the layout (`config/`, `logs/`, `assets/`).
    #[allow(dead_code)]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Folder holding mutable application data (`device.json`, keys, ...).
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// Folder holding rotating logs and crash reports.
    pub fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }

    /// Folder holding read-only assets shipped with the release.
    pub fn assets_dir(&self) -> &Path {
        &self.assets_dir
    }

    /// Directory that holds generated device keypairs.
    pub fn keys_dir(&self) -> PathBuf {
        self.config_dir.join("keys")
    }

    // ── Files ---------------------------------------------------------------

    /// Persistent device identity (`device.json`).
    #[allow(dead_code)]
    pub fn identity_file(&self) -> PathBuf {
        self.config_dir.join("device.json")
    }

    /// Persisted trusted-device store (`trusted.json`).
    #[allow(dead_code)]
    pub fn trusted_file(&self) -> PathBuf {
        self.config_dir.join("trusted.json")
    }

    /// Editable application configuration (`unikey.toml`).
    pub fn app_config_file(&self) -> PathBuf {
        self.config_dir.join("unikey.toml")
    }

    /// Machine-readable build metadata written at startup (`version.json`).
    /// Lives at the top of the layout so tooling (and the future auto-updater)
    /// can find it without knowing where mutable config is stored.
    pub fn version_file(&self) -> PathBuf {
        self.root.join("version.json")
    }

    /// The actively written log file.
    pub fn active_log_file(&self) -> PathBuf {
        self.logs_dir.join("latest.log")
    }

    /// Crash report written by the panic hook.
    pub fn panic_log_file(&self) -> PathBuf {
        self.logs_dir.join("panic.log")
    }

    // ── Bootstrap ------------------------------------------------------------

    /// Create every directory the daemon needs. Safe to call on every start;
    /// existing state is left untouched.
    pub fn initialize(&self) -> Result<()> {
        for dir in [
            self.config_dir(),
            self.logs_dir(),
            self.assets_dir(),
            &self.keys_dir(),
        ] {
            fs::create_dir_all(dir)
                .with_context(|| format!("unable to create directory {}", dir.display()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique, fresh temporary directory per test (tests run in parallel).
    fn temp_dir(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!("unikey-paths-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).expect("create temp base");
        base
    }

    #[test]
    fn portable_layout_is_fully_relative() {
        let root = temp_dir("portable");
        fs::write(root.join(PORTABLE_MARKER), "portable\n").expect("marker");

        let paths = RuntimePaths::detect_from(&root).expect("detect");
        assert_eq!(paths.mode(), PathMode::Portable);
        assert_eq!(paths.config_dir(), root.join("config"));
        assert_eq!(paths.logs_dir(), root.join("logs"));
        assert_eq!(paths.assets_dir(), root.join("assets"));
        assert_eq!(paths.identity_file(), root.join("config/device.json"));
        assert_eq!(paths.trusted_file(), root.join("config/trusted.json"));
        assert_eq!(paths.version_file(), root.join("version.json"));
        assert_eq!(paths.keys_dir(), root.join("config/keys"));
        assert_eq!(paths.active_log_file(), root.join("logs/latest.log"));
        assert_eq!(paths.panic_log_file(), root.join("logs/panic.log"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn standard_layout_uses_os_config_dir() {
        // Cannot easily override dirs::config_dir, so assert the shape only.
        let base = temp_dir("standard");
        fs::create_dir_all(&base).expect("base");
        // Construction must succeed regardless of the OS config dir value.
        let paths = RuntimePaths::detect_from(&base.join("some-exe-dir")).expect("detect");
        assert_eq!(paths.mode(), PathMode::Standard);
        assert!(paths.config_dir().ends_with("UniKey"));
        assert!(paths.logs_dir().ends_with("UniKey/logs"));
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn initialize_creates_every_directory() {
        let root = temp_dir("init");
        fs::write(root.join(PORTABLE_MARKER), "portable\n").expect("marker");
        let paths = RuntimePaths::detect_from(&root).expect("detect");

        paths.initialize().expect("initialize");
        for dir in [
            paths.config_dir(),
            paths.logs_dir(),
            paths.assets_dir(),
            &paths.keys_dir(),
        ] {
            assert!(dir.is_dir(), "expected {} to exist", dir.display());
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn no_marker_means_standard() {
        let root = temp_dir("nomarker");
        fs::create_dir_all(&root).expect("root");
        let paths = RuntimePaths::detect_from(&root).expect("detect");
        assert_eq!(paths.mode(), PathMode::Standard);
        let _ = fs::remove_dir_all(&root);
    }
}
