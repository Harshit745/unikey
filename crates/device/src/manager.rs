use anyhow::Result;

use crate::{
    device_info::DeviceInfo,
    identity::DeviceIdentity,
    paths::Paths,
    repository::Repository,
};

/// Owns the device identity lifecycle (load, create, persist).
pub struct DeviceManager {
    repository: Repository,
}

impl DeviceManager {
    /// Build a manager rooted at the OS config directory.
    pub fn new() -> Result<Self> {
        Ok(Self {
            repository: Repository::new()?,
        })
    }

    /// Build a manager rooted at an explicitly supplied directory (portable
    /// release folder, tests). Identity and keypair live under that root.
    pub fn new_with_paths(paths: Paths) -> Result<Self> {
        Ok(Self {
            repository: Repository::new_with_paths(paths),
        })
    }

    /// Load the existing identity, or generate, persist and return a new one.
    pub fn load_or_create(&self) -> anyhow::Result<DeviceIdentity> {
        if self.repository.exists() {
            return self.repository.load();
        }

        let (private_key, public_key) = crypto::generate_keypair()?;

        crypto::save_keypair(
            &private_key,
            &public_key,
            self.repository.private_key_path(),
            self.repository.public_key_path(),
        )?;

        let identity = DeviceIdentity {
            id: uuid::Uuid::new_v4(),
            name: DeviceInfo::hostname(),
            platform: DeviceInfo::platform(),
            device_type: DeviceInfo::device_type(),
            created_at: chrono::Utc::now(),
            protocol_version: 1,

            public_key_path: self
                .repository
                .public_key_path()
                .display()
                .to_string(),

            private_key_path: self
                .repository
                .private_key_path()
                .display()
                .to_string(),
        };

        self.repository.save(&identity)?;

        Ok(identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_or_create_with_injected_root() {
        let base = std::env::temp_dir().join(format!("unikey-device-{}", std::process::id()));
        let root = base.join("data");
        let _ = std::fs::remove_dir_all(&base);

        let paths = Paths::with_root(root.clone()).expect("paths");
        let manager = DeviceManager::new_with_paths(paths).expect("manager");
        let identity = manager.load_or_create().expect("create");

        assert_eq!(identity.name, DeviceInfo::hostname());
        assert!(root.join("device.json").exists(), "identity file created");
        assert!(root.join("keys/public.key").exists(), "public key created");
        assert!(root.join("keys/private.key").exists(), "private key created");

        // A second manager on the same root must load the same identity.
        let paths2 = Paths::with_root(root).expect("paths2");
        let manager2 = DeviceManager::new_with_paths(paths2).expect("manager2");
        let identity2 = manager2.load_or_create().expect("load");
        assert_eq!(identity.id, identity2.id, "identity is stable across restarts");

        let _ = std::fs::remove_dir_all(&base);
    }
}
