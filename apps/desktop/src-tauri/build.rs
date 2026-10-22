//! Build script for the UniKey desktop shell.
//!
//! Besides `tauri_build`, this emits a `unikey_embed_daemon` cfg when an
//! embedded daemon binary is present next to this crate. The packaging build
//! drops `target/release/daemon` there as `unikey-daemon.bin`; development
//! builds compile without one.

use std::path::Path;

fn main() {
    // Register the custom cfg so the compiler doesn't warn about it.
    println!("cargo:rustc-check-cfg=cfg(unikey_embed_daemon)");

    // Emit the embed cfg only if a daemon binary is staged for embedding.
    let embed_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("unikey-daemon.bin");
    if embed_path.exists() {
        println!("cargo:rustc-cfg=unikey_embed_daemon");
        println!("cargo:rerun-if-changed={}", embed_path.display());
    }

    tauri_build::build()
}
