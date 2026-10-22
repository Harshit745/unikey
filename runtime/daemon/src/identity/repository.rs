use std::fs;
use std::io;

use crate::identity::{ConfigPaths, DeviceIdentity};

pub struct IdentityRepository {
    paths: ConfigPaths,
}

impl IdentityRepository {
    pub fn new(paths: ConfigPaths) -> Self {
        Self { paths }
    }

    pub fn exists(&self) -> bool {
        self.paths.device_file().exists()
    }

    pub fn load(&self) -> io::Result<DeviceIdentity> {
        let json = fs::read_to_string(self.paths.device_file())?;

        let identity = serde_json::from_str(&json)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(identity)
    }

    pub fn save(&self, identity: &DeviceIdentity) -> io::Result<()> {
        self.paths.ensure()?;

        let json = serde_json::to_string_pretty(identity)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        fs::write(self.paths.device_file(), json)?;

        Ok(())
    }

    pub fn delete(&self) -> io::Result<()> {
        if self.exists() {
            fs::remove_file(self.paths.device_file())?;
        }

        Ok(())
    }
}