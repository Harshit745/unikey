use crate::identity::{DeviceType, Platform};

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub device_name: String,
    pub device_type: DeviceType,
    pub platform: Platform,
}

impl DeviceInfo {
    pub fn current() -> Self {
        let device_name = hostname::get()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();

        Self {
            device_name,
            device_type: DeviceType::Unknown,
            platform: Platform::current(),
        }
    }
}