use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

use anyhow::Result;

/// A device that has been successfully paired and trusted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedDevice {
    pub id: Uuid,
    pub name: String,
    /// Hex-encoded Ed25519 public key pinned during pairing. Older trust-store
    /// entries deserialize with an empty value and must be paired again before
    /// protected services accept traffic from them.
    #[serde(default)]
    pub identity_public_key: String,
    /// Platform identifier (macOS, Windows, Linux).
    #[serde(default)]
    pub platform: String,
    /// Last time this device was seen online.
    #[serde(default = "Utc::now")]
    pub last_seen: DateTime<Utc>,
    /// Human-readable label assigned by the user (independent of device name).
    #[serde(default)]
    pub alias: Option<String>,
}

/// Outcome of a pairing interaction, persisted for the history view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairingEventKind {
    Paired,
    Removed,
    Rejected,
    Expired,
}

/// One entry in the pairing history log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingEvent {
    pub kind: PairingEventKind,
    pub peer_id: Uuid,
    pub peer_name: String,
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
}

/// Persisted store of all devices that this daemon has paired with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustStore {
    pub trusted: Vec<TrustedDevice>,
    /// Chronological log of pairing interactions (bounded).
    #[serde(default)]
    pub history: Vec<PairingEvent>,
    #[serde(skip)]
    path: PathBuf,
}

impl TrustStore {
    /// Load the trust store from `~/.config/UniKey/trusted.json`.
    /// Creates an empty store if the file does not exist.
    pub fn load() -> Result<Self> {
        let mut root = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        root.push("UniKey");
        Self::load_at(&root)
    }

    /// Load the trust store from `root/trusted.json` where `root` is an
    /// explicitly supplied data directory (portable release folder, tests).
    /// Creates an empty store if the file does not exist.
    pub fn load_at(root: &std::path::Path) -> Result<Self> {
        let path = root.join("trusted.json");

        let store = if path.exists() {
            let json = fs::read_to_string(&path)?;
            let mut store: Self = serde_json::from_str(&json)?;
            store.path = path;
            store
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            Self {
                trusted: Vec::new(),
                history: Vec::new(),
                path,
            }
        };
        Ok(store)
    }

    /// Persist the current trust store to disk.
    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self)?;
        fs::write(&self.path, json)?;
        Ok(())
    }

    /// Add a device to the trust store (idempotent).
    pub fn add_trusted(
        &mut self,
        id: Uuid,
        name: &str,
        platform: &str,
        identity_public_key: &[u8; 32],
    ) {
        let key_hex = identity_public_key
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if !self.trusted.iter().any(|d| d.id == id) {
            self.trusted.push(TrustedDevice {
                id,
                name: name.to_string(),
                identity_public_key: key_hex,
                platform: platform.to_string(),
                last_seen: Utc::now(),
                alias: None,
            });
        } else {
            // Update last_seen and name for existing devices
            if let Some(d) = self.trusted.iter_mut().find(|d| d.id == id) {
                d.last_seen = Utc::now();
                d.name = name.to_string();
                d.identity_public_key = key_hex;
                if !platform.is_empty() {
                    d.platform = platform.to_string();
                }
            }
        }
    }

    /// Remove a device from the trust store.
    pub fn remove_trusted(&mut self, id: &Uuid) {
        self.trusted.retain(|d| &d.id != id);
    }

    /// Rename a trusted device (user-assigned alias).
    pub fn rename_device(&mut self, id: &Uuid, new_alias: &str) -> bool {
        if let Some(d) = self.trusted.iter_mut().find(|d| &d.id == id) {
            d.alias = Some(new_alias.to_string());
            true
        } else {
            false
        }
    }

    /// Check whether a device ID is trusted.
    pub fn is_trusted(&self, id: &Uuid) -> bool {
        self.trusted.iter().any(|d| &d.id == id)
    }

    /// Check the device UUID and the authenticated Ed25519 identity key.
    pub fn is_trusted_identity(&self, id: &Uuid, identity_public_key: &[u8; 32]) -> bool {
        let key_hex = identity_public_key
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        self.trusted
            .iter()
            .any(|device| &device.id == id && device.identity_public_key == key_hex)
    }

    /// Append a pairing event to the history log (bounded to the last 200).
    pub fn record_event(
        &mut self,
        kind: PairingEventKind,
        peer_id: Uuid,
        peer_name: &str,
    ) {
        self.history.push(PairingEvent {
            kind,
            peer_id,
            peer_name: peer_name.to_string(),
            timestamp: Utc::now(),
        });
        if self.history.len() > 200 {
            let excess = self.history.len() - 200;
            self.history.drain(..excess);
        }
    }

    /// Return the pairing history, most recent first.
    pub fn history(&self) -> Vec<PairingEvent> {
        let mut events = self.history.clone();
        events.reverse();
        events
    }

    /// Return a list of all trusted devices.
    pub fn list(&self) -> &[TrustedDevice] {
        &self.trusted
    }

    /// Return the number of trusted devices.
    pub fn count(&self) -> usize {
        self.trusted.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh per-test directory, cleaned up on drop.
    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "unikey-trust-{tag}-{}",
            Uuid::new_v4()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn load_at_creates_an_empty_store() {
        let root = temp_root("empty");
        let store = TrustStore::load_at(&root).expect("load");
        assert_eq!(store.count(), 0);
        assert!(store.history().is_empty());
    }

    #[test]
    fn add_is_idempotent_and_refreshes_last_seen() {
        let root = temp_root("add");
        let mut store = TrustStore::load_at(&root).unwrap();
        let id = Uuid::new_v4();

        store.add_trusted(id, "bob", "macOS", &[1; 32]);
        store.add_trusted(id, "bob", "macOS", &[1; 32]);
        assert_eq!(store.count(), 1);

        // Re-adding refreshes the name/platform.
        store.add_trusted(id, "robert", "Windows", &[2; 32]);
        let d = store.list().iter().find(|d| d.id == id).unwrap();
        assert_eq!(d.name, "robert");
        assert_eq!(d.platform, "Windows");
        assert!(store.is_trusted_identity(&id, &[2; 32]));
        assert!(!store.is_trusted_identity(&id, &[1; 32]));
    }

    #[test]
    fn trusted_and_untrusted_are_distinguished() {
        let root = temp_root("trusted");
        let mut store = TrustStore::load_at(&root).unwrap();
        let id = Uuid::new_v4();
        store.add_trusted(id, "alice", "Linux", &[3; 32]);
        assert!(store.is_trusted(&id));
        assert!(!store.is_trusted(&Uuid::new_v4()));
    }

    #[test]
    fn remove_and_rename() {
        let root = temp_root("rename");
        let mut store = TrustStore::load_at(&root).unwrap();
        let id = Uuid::new_v4();
        store.add_trusted(id, "dev", "Linux", &[4; 32]);

        assert!(store.rename_device(&id, "work-laptop"));
        assert!(!store.rename_device(&Uuid::new_v4(), "nope"));

        store.remove_trusted(&id);
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn save_then_reload_round_trips() {
        let root = temp_root("persist");
        let id = Uuid::new_v4();
        {
            let mut store = TrustStore::load_at(&root).unwrap();
            store.add_trusted(id, "carol", "macOS", &[5; 32]);
            store.record_event(PairingEventKind::Paired, id, "carol");
            store.save().unwrap();
        }
        // A fresh load from the same root must see the persisted data.
        let reloaded = TrustStore::load_at(&root).unwrap();
        assert_eq!(reloaded.count(), 1);
        assert!(reloaded.is_trusted(&id));
        assert!(reloaded.is_trusted_identity(&id, &[5; 32]));
        let events = reloaded.history();
        let ev = events.first().unwrap();
        assert_eq!(ev.kind, PairingEventKind::Paired);
        assert_eq!(ev.peer_id, id);
    }

    #[test]
    fn history_is_bounded_and_most_recent_first() {
        let root = temp_root("history");
        let mut store = TrustStore::load_at(&root).unwrap();
        let id = Uuid::new_v4();
        for i in 0..250 {
            store.record_event(
                PairingEventKind::Paired,
                id,
                &format!("peer-{i}"),
            );
        }
        // Bounded to the last 200 entries.
        assert_eq!(store.history.len(), 200);
        let hist = store.history();
        assert_eq!(hist.len(), 200);
        // Most recent first.
        assert_eq!(hist[0].peer_name, "peer-249");
        assert_eq!(hist[199].peer_name, "peer-50");
    }

    #[test]
    fn corrupted_store_is_reported_as_an_error() {
        let root = temp_root("corrupt");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("trusted.json"), "{ not valid json").unwrap();
        // The caller (daemon bootstrap) handles this explicitly rather than
        // silently wiping user trust data.
        assert!(TrustStore::load_at(&root).is_err());
    }
}
