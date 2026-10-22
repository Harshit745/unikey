use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Resolves where the device identity and keypair live.
///
/// The default root is the OS per-user config directory (`UniKey/`), which
/// preserves the historical layout. The daemon may inject an explicit root
/// (e.g. a portable release folder) via [`Paths::with_root`] without changing
/// any business logic — path handling is kept out of identity/crypto code.
#[derive(Debug, Clone)]
pub struct Paths {
    root: PathBuf,
}

impl Default for Paths {
    fn default() -> Self {
        Self {
            root: PathBuf::new(),
        }
    }
}

impl Paths {
    /// Build paths rooted at the OS per-user config directory (`UniKey/`).
    pub fn new() -> Result<Self> {
        let mut root = dirs::config_dir().context("Unable to locate config directory")?;
        root.push("UniKey");
        Self::with_root(root)
    }

    /// Build paths rooted at an explicitly supplied directory (portable mode,
    /// tests, or an installer-managed location). Creates the keys directory.
    pub fn with_root(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(root.join("keys"))?;
        Ok(Self { root })
    }

    /// The root directory holding identity state.
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn identity_file(&self) -> PathBuf {
        self.root.join("device.json")
    }

    pub fn public_key_file(&self) -> PathBuf {
        self.root.join("keys/public.key")
    }

    pub fn private_key_file(&self) -> PathBuf {
        self.root.join("keys/private.key")
    }
}
