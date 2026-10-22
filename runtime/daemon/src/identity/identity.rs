use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::identity::{DeviceType, Platform};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceIdentity {
    /// Permanent unique identifier for this installation.
    pub id: Uuid,

    /// Human-readable device name.
    pub device_name: String,

    /// Desktop, Laptop, Phone, etc.
    pub device_type: DeviceType,

    /// Current operating system.
    pub platform: Platform,

    /// When this identity was first created.
    pub created_at: DateTime<Utc>,

    /// UniKey protocol version.
    pub protocol_version: u32,

    /// Relative path to the public key.
    pub public_key_path: String,

    /// Relative path to the private key.
    pub private_key_path: String,
}