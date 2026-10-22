//! Build metadata baked in by [`../build.rs`], surfaced via `version.json`
//! and the `/api/v1/health` endpoint.
//!
//! All fields fall back to sensible defaults so the daemon still compiles and
//! runs when the metadata is absent (e.g. `cargo run` from a non-git tree).

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::Serialize;

/// Version string shared by every crate in the workspace.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Short git hash of the commit this binary was built from.
pub const COMMIT: &str = match option_env!("UNIKEY_COMMIT") {
    Some(commit) => commit,
    None => "unknown",
};

/// Build timestamp as a unix epoch (seconds since 1970-01-01T00:00:00Z),
/// or `0` when the build script did not capture one.
pub fn build_epoch() -> u64 {
    option_env!("UNIKEY_BUILD_EPOCH")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0)
}

/// Human-readable build time. Falls back to "unknown" when not captured.
pub fn build_date() -> String {
    let epoch = build_epoch();
    if epoch == 0 {
        return "unknown".to_string();
    }
    DateTime::<Utc>::from_timestamp(epoch as i64, 0)
        .map(|d| d.to_rfc3339())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Whether this binary was built with optimisations (release).
pub const PROFILE: &str = if cfg!(debug_assertions) { "debug" } else { "release" };

/// Platform this binary targets, as a stable lowercase string.
pub fn platform() -> &'static str {
    std::env::consts::OS
}

/// Machine-readable build information, serialised to `version.json` and
/// returned by the health endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct BuildInfo {
    pub version: &'static str,
    pub build: String,
    pub commit: &'static str,
    pub platform: &'static str,
    pub profile: &'static str,
    pub rustc: String,
}

impl BuildInfo {
    /// Capture the build metadata of this binary.
    pub fn capture() -> Self {
        Self {
            version: VERSION,
            build: build_date(),
            commit: COMMIT,
            platform: platform(),
            profile: PROFILE,
            rustc: option_env!("UNIKEY_RUSTC").unwrap_or("unknown").to_string(),
        }
    }

    /// Write `version.json` to `path` (creating any missing parents).
    ///
    /// Best effort: a failure here is logged, not fatal — version metadata is
    /// diagnostic, never a runtime requirement.
    pub fn write_version_file(&self, path: &Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_sane_version() {
        let info = BuildInfo::capture();
        assert_eq!(info.version, VERSION);
        assert!(!info.version.is_empty());
        assert!(!info.platform.is_empty());
    }

    #[test]
    fn platform_is_stable_lowercase() {
        // std::env::consts::OS yields "macos", "windows", "linux", ...
        let p = platform();
        assert!(!p.is_empty());
        assert_eq!(p, p.to_lowercase());
    }

    #[test]
    fn version_file_is_valid_json() {
        let dir = std::env::temp_dir().join(format!("unikey-version-test-{}", std::process::id()));
        let path = dir.join("version.json");
        let info = BuildInfo::capture();
        info.write_version_file(&path).expect("write should succeed");

        let text = std::fs::read_to_string(&path).expect("read should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        assert_eq!(parsed["version"], VERSION);
        assert!(parsed["commit"].is_string());
        assert!(parsed["platform"].is_string());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
