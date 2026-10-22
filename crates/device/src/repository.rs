use anyhow::Result;
use std::fs;
use std::path::PathBuf;

use crate::{identity::DeviceIdentity, paths::Paths};

/// Persists and loads the device identity on disk.
pub struct Repository {
    paths: Paths,
}

impl Repository {
    /// Build a repository rooted at the OS config directory.
    pub fn new() -> Result<Self> {
        Ok(Self {
            paths: Paths::new()?,
        })
    }

    /// Build a repository rooted at an explicitly supplied directory
    /// (portable release folder, tests, installer-managed locations).
    pub fn new_with_paths(paths: Paths) -> Self {
        Self { paths }
    }

    pub fn exists(&self) -> bool {
        self.paths.identity_file().exists()
    }

    pub fn load(&self) -> Result<DeviceIdentity> {
        let json = fs::read_to_string(self.paths.identity_file())?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn save(&self, identity: &DeviceIdentity) -> Result<()> {
        let json = serde_json::to_string_pretty(identity)?;
        fs::write(self.paths.identity_file(), json)?;
        Ok(())
    }

    pub fn public_key_path(&self) -> PathBuf {
        self.paths.public_key_file()
    }

    pub fn private_key_path(&self) -> PathBuf {
        self.paths.private_key_file()
    }

    /// The root directory holding identity state.
    pub fn root(&self) -> &std::path::Path {
        self.paths.root()
    }
}
