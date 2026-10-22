//! Build script for the UniKey daemon.
//!
//! Captures two pieces of build metadata and bakes them into the binary:
//!
//!   * `UNIKEY_COMMIT` — short git commit hash of `HEAD` (or the
//!     `UNIKEY_OVERRIDE_COMMIT` env var, for reproducible/CI builds).
//!   * `UNIKEY_BUILD_EPOCH` — unix timestamp of the build. CI can pin this
//!     via the standard `SOURCE_DATE_EPOCH` variable for reproducible builds.
//!
//! Neither is guaranteed to exist at compile time, so the runtime code always
//! reads them with [`option_env!`] and falls back to sensible defaults.

use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    // Re-run only when these change. `.git/HEAD` is tracked so branch switches
    // pick up a fresh commit hash; individual commits on a branch require a
    // clean rebuild (the release pipeline always clean-builds).
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", git_head_path().display());
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
    println!("cargo:rerun-if-env-changed=UNIKEY_OVERRIDE_COMMIT");

    // --- Commit hash -----------------------------------------------------
    let commit = std::env::var("UNIKEY_OVERRIDE_COMMIT")
        .ok()
        .or_else(git_short_hash)
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=UNIKEY_COMMIT={commit}");

    // --- Build timestamp --------------------------------------------------
    let epoch = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs()))
        .unwrap_or(0);
    println!("cargo:rustc-env=UNIKEY_BUILD_EPOCH={epoch}");

    // --- Compiler version --------------------------------------------------
    // Record the exact `rustc --version` string so `version.json`/health can
    // identify the toolchain that produced this binary.
    let rustc = Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string()))
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=UNIKEY_RUSTC={rustc}");
}

/// Absolute path to the repository's `.git/HEAD`, used for change tracking.
fn git_head_path() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()))
        .join(".git")
        .join("HEAD")
}

/// Return the short git hash of the current HEAD, or `None` if git is
/// unavailable or this is not a git work tree.
fn git_short_hash() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if hash.is_empty() || hash == "HEAD" {
        None
    } else {
        Some(hash)
    }
}
