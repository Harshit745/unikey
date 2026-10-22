//! Editable application configuration (`unikey.toml`).
//!
//! Guarantees:
//!   * Missing file → a default configuration is created and returned.
//!   * Malformed file → a warning is logged and defaults are returned; the
//!     user's file is **never overwritten or deleted**.
//!   * Wrong types → serde rejects the field and the same fallback applies.
//!
//! The daemon therefore never crashes because configuration is absent or
//! broken. Defaults mirror the protocol constants in `crates/common`.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Version of the config *schema*. Bump when fields are renamed or removed so
/// older configs can be migrated explicitly rather than silently ignored.
pub const CONFIG_SCHEMA_VERSION: &str = "1.0";

/// Default TCP port for the device transport.
pub const DEFAULT_TCP_PORT: u16 = common::constants::DEFAULT_TCP_PORT;

/// Default UDP port for discovery beacons.
pub const DEFAULT_DISCOVERY_PORT: u16 = common::constants::DEFAULT_DISCOVERY_PORT;

/// Default localhost port for the HTTP API.
pub const DEFAULT_API_PORT: u16 = crate::api::API_PORT;

/// Root configuration of the daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Config schema version (not the application version).
    pub schema_version: String,
    pub daemon: DaemonConfig,
    pub network: NetworkConfig,
    pub logging: LoggingConfig,
    pub services: ServicesConfig,
    /// Clipboard sync behaviour.
    #[serde(default)]
    pub clipboard: ClipboardConfig,
    /// Keyboard & mouse forwarding behaviour.
    #[serde(default)]
    pub input: InputConfig,
    /// File transfer behaviour.
    #[serde(default)]
    pub transfer: TransferConfig,
    /// Transport encryption knobs.
    #[serde(default)]
    pub encryption: EncryptionConfig,
}

/// Clipboard sync settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClipboardConfig {
    /// Broadcast clipboard changes to connected peers.
    pub auto_sync: bool,
    /// Also sync image clipboard content (not just text/HTML).
    pub sync_images: bool,
    /// How many recent clipboard entries the API/GUI keeps.
    pub history_limit: usize,
    /// Polling interval in milliseconds for the local clipboard watcher.
    pub poll_interval_ms: u64,
}

/// Keyboard & mouse forwarding settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputConfig {
    /// Allow remote keyboard events to be injected.
    pub keyboard_forwarding: bool,
    /// Allow remote mouse events to be injected.
    pub mouse_forwarding: bool,
    /// Enable shared-cursor edge switching between screens.
    pub edge_switching: bool,
}

/// File transfer settings.
///
/// `Default` (derived) yields `max_file_size: 0` → protocol maximum (8 GiB),
/// so the default never silently rejects large-but-legitimate files, and
/// `download_dir: None` → the daemon's default downloads folder.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferConfig {
    /// Maximum accepted file size in bytes (0 → protocol maximum).
    pub max_file_size: u64,
    /// Where received files are saved. `null` → default downloads folder.
    pub download_dir: Option<std::path::PathBuf>,
}

/// Transport encryption settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncryptionConfig {
    /// Re-key the session after this many minutes (0 → never).
    pub key_rotation_minutes: u64,
    /// Replay-guard window in milliseconds for inbound frames.
    pub max_replay_window_ms: u64,
    /// Require a successful pairing before any transport session is accepted.
    pub require_pairing: bool,
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            auto_sync: true,
            sync_images: true,
            history_limit: 100,
            poll_interval_ms: 250,
        }
    }
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            keyboard_forwarding: true,
            mouse_forwarding: true,
            edge_switching: false,
        }
    }
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            key_rotation_minutes: 0, // keep the session key for its lifetime
            max_replay_window_ms: 30_000,
            require_pairing: true,
        }
    }
}

/// Daemon-level settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonConfig {
    /// Optional display name override. `null` → derive from hostname.
    pub device_name: Option<String>,
}

/// Network / ports. Changing these affects all peers on the LAN.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkConfig {
    /// Bind address for the TCP transport.
    pub tcp_bind: String,
    /// TCP port used by the device transport.
    pub tcp_port: u16,
    /// UDP port used by discovery beacons.
    pub discovery_port: u16,
    /// Localhost port for the HTTP API.
    pub api_port: u16,
    /// Bind address for UDP discovery beacons.
    pub discovery_bind: String,
}

/// Logging behaviour.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoggingConfig {
    /// One of: trace, debug, info, warn, error.
    pub level: String,
    /// Rotate `latest.log` once it exceeds this many bytes.
    pub max_bytes: u64,
    /// Keep at most this many rotated backups.
    pub max_backups: usize,
    /// Write a file to `logs/` in addition to the console.
    pub file_enabled: bool,
}

/// Per-service toggles. Future modules (clipboard, input, files, ...) slot in
/// here without changes to the pipeline or the config loader.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServicesConfig {
    pub clipboard: bool,
    pub input: bool,
    pub files: bool,
    pub pairing: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION.to_string(),
            daemon: DaemonConfig { device_name: None },
            network: NetworkConfig {
                tcp_bind: "0.0.0.0".to_string(),
                tcp_port: DEFAULT_TCP_PORT,
                discovery_port: DEFAULT_DISCOVERY_PORT,
                api_port: DEFAULT_API_PORT,
                discovery_bind: "0.0.0.0".to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                max_bytes: 5 * 1024 * 1024, // 5 MiB
                max_backups: 5,
                file_enabled: true,
            },
            services: ServicesConfig {
                clipboard: true,
                input: true,
                files: true,
                pairing: true,
            },
            clipboard: ClipboardConfig::default(),
            input: InputConfig::default(),
            transfer: TransferConfig::default(),
            encryption: EncryptionConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from `path`, creating a default file when missing.
    ///
    /// Never fails because the file is absent or malformed — the returned
    /// [`Config`] is always valid (or a hard I/O error is reported).
    pub fn load_or_create(path: &Path) -> Result<Self> {
        if !path.exists() {
            let config = Self::default();
            config.persist(path).with_context(|| {
                format!("unable to write default configuration to {}", path.display())
            })?;
            eprintln!(
                "Created default configuration at {} (edit it to customise UniKey)",
                path.display()
            );
            return Ok(config);
        }

        let text = fs::read_to_string(path)
            .with_context(|| format!("unable to read configuration {}", path.display()))?;

        match toml::from_str::<Config>(&text) {
            Ok(config) => Ok(config),
            Err(err) => {
                // Malformed config: warn loudly but keep the user's file intact
                // and fall back to defaults so the daemon still starts.
                eprintln!(
                    "WARNING: configuration {} is invalid and was ignored: {err}\n\
                     Falling back to built-in defaults.",
                    path.display()
                );
                Ok(Self::default())
            }
        }
    }

    /// Serialise this configuration to `path` as pretty TOML.
    pub fn persist(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self)?;
        fs::write(path, text)?;
        Ok(())
    }

    /// Best-effort sanity check of the network ports. Returns a warning list;
    /// a zero or colliding port is logged at call sites, not fatal.
    pub fn port_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let ports = [
            ("tcp_port", self.network.tcp_port),
            ("discovery_port", self.network.discovery_port),
            ("api_port", self.network.api_port),
        ];
        for (name, port) in ports {
            if port == 0 {
                warnings.push(format!("{name} is 0, which disables the listener"));
            }
        }
        if self.network.tcp_port == self.network.discovery_port
            || self.network.tcp_port == self.network.api_port
            || self.network.discovery_port == self.network.api_port
        {
            warnings.push("two or more network ports are identical".to_string());
        }
        warnings
    }

    /// Best-effort sanity check of the feature settings. Returns a warning
    /// list (never fatal) for values that are legal but almost certainly
    /// mistakes.
    pub fn settings_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.clipboard.poll_interval_ms < 50 {
            warnings.push(format!(
                "clipboard.poll_interval_ms={} is below the 50ms floor; the watcher may spin",
                self.clipboard.poll_interval_ms
            ));
        }
        if self.clipboard.history_limit == 0 {
            warnings.push("clipboard.history_limit=0 disables clipboard history".to_string());
        }
        if self.input.keyboard_forwarding && !self.input.mouse_forwarding
            || self.input.mouse_forwarding && !self.input.keyboard_forwarding
        {
            warnings.push(
                "only one of input.keyboard_forwarding / input.mouse_forwarding is enabled; \
                 enabling both gives the smoothest remote control"
                    .to_string(),
            );
        }
        if self.transfer.max_file_size > 0 && self.transfer.max_file_size < 1024 {
            warnings.push(format!(
                "transfer.max_file_size={} is smaller than 1KiB",
                self.transfer.max_file_size
            ));
        }
        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!("unikey-config-{}", std::process::id()));
        let _ = fs::create_dir_all(&base);
        base.join(name)
    }

    #[test]
    fn defaults_are_sane() {
        let c = Config::default();
        assert_eq!(c.network.tcp_port, DEFAULT_TCP_PORT);
        assert_eq!(c.network.discovery_port, DEFAULT_DISCOVERY_PORT);
        assert_eq!(c.network.api_port, DEFAULT_API_PORT);
        assert!(c.logging.max_bytes > 0);
        assert!(c.services.clipboard && c.services.pairing);
        assert!(c.port_warnings().is_empty());
    }

    #[test]
    fn creates_default_when_missing() {
        let path = temp_file("missing.toml");
        let _ = fs::remove_file(&path);
        let c = Config::load_or_create(&path).expect("load");
        assert!(path.exists(), "default config file should be created");
        assert_eq!(c.network.tcp_port, DEFAULT_TCP_PORT);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn malformed_file_falls_back_without_overwrite() {
        let path = temp_file("broken.toml");
        fs::write(&path, "this is { not valid toml [ [ [").expect("write");
        let original = fs::read_to_string(&path).expect("read");
        let c = Config::load_or_create(&path).expect("load");
        assert_eq!(c.network.tcp_port, DEFAULT_TCP_PORT, "defaults returned");
        let after = fs::read_to_string(&path).expect("read");
        assert_eq!(original, after, "user's broken file must not be clobbered");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn round_trips_through_persist() {
        let path = temp_file("roundtrip.toml");
        let c = Config::default();
        c.persist(&path).expect("persist");
        let loaded = Config::load_or_create(&path).expect("load");
        assert_eq!(loaded.network.tcp_port, c.network.tcp_port);
        assert_eq!(loaded.logging.level, c.logging.level);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn colliding_ports_warn() {
        let mut c = Config::default();
        c.network.tcp_port = 1000;
        c.network.discovery_port = 1000;
        assert!(!c.port_warnings().is_empty());
    }

    #[test]
    fn feature_sections_have_sane_defaults() {
        let c = Config::default();
        assert!(c.clipboard.auto_sync);
        assert_eq!(c.clipboard.poll_interval_ms, 250);
        assert!(c.input.keyboard_forwarding && c.input.mouse_forwarding);
        assert_eq!(c.transfer.max_file_size, 0, "0 means protocol maximum");
        assert!(c.transfer.download_dir.is_none());
        assert!(c.encryption.require_pairing);
        assert!(c.settings_warnings().is_empty());
    }

    #[test]
    fn old_config_without_feature_sections_still_loads() {
        // A config written before the feature sections existed must parse to
        // defaults for the new sections (backward compatible).
        let toml_text = r#"
            schema_version = "1.0"

            [daemon]
            device_name = "alice"

            [network]
            tcp_bind = "0.0.0.0"
            tcp_port = 45892
            discovery_port = 45891
            api_port = 45895
            discovery_bind = "0.0.0.0"

            [logging]
            level = "info"
            max_bytes = 1000
            max_backups = 2
            file_enabled = true

            [services]
            clipboard = true
            input = true
            files = true
            pairing = true
        "#;
        let c: Config = toml::from_str(toml_text).expect("parses");
        assert_eq!(c.daemon.device_name.as_deref(), Some("alice"));
        assert!(c.clipboard.auto_sync, "new section defaults in");
        assert_eq!(c.transfer.download_dir, None);
        assert!(c.encryption.require_pairing);
    }

    #[test]
    fn settings_warnings_catch_obvious_mistakes() {
        let mut c = Config::default();
        c.clipboard.poll_interval_ms = 10;
        assert!(!c.settings_warnings().is_empty());

        let mut c = Config::default();
        c.transfer.max_file_size = 64;
        assert!(!c.settings_warnings().is_empty());
    }
}
