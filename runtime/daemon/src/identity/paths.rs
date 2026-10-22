use std::fs;
use std::io;
use std::path::PathBuf;

pub struct ConfigPaths {
    root: PathBuf,
}

impl ConfigPaths {
    pub fn new() -> io::Result<Self> {
        let mut root = dirs::config_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Config directory not found"))?;

        root.push("UniKey");

        fs::create_dir_all(&root)?;

        Ok(Self { root })
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    pub fn device_file(&self) -> PathBuf {
        self.root.join("device.json")
    }

    pub fn keys_dir(&self) -> PathBuf {
        self.root.join("keys")
    }

    pub fn public_key(&self) -> PathBuf {
        self.keys_dir().join("public.key")
    }

    pub fn private_key(&self) -> PathBuf {
        self.keys_dir().join("private.key")
    }

    pub fn ensure(&self) -> io::Result<()> {
        fs::create_dir_all(self.keys_dir())?;
        Ok(())
    }
}