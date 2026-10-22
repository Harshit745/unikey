use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{DeviceType, Platform};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceIdentity {
    pub id: Uuid,
    pub name: String,
    pub platform: Platform,
    pub device_type: DeviceType,
    pub created_at: DateTime<Utc>,
    pub protocol_version: u32,

    pub public_key_path: String,
    pub private_key_path: String,
}