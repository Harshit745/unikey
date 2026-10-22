use crate::{DeviceType, Platform};

pub struct DeviceInfo;

impl DeviceInfo {
    pub fn hostname() -> String {
        hostname::get()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    }

    pub fn platform() -> Platform {
        Platform::current()
    }

    pub fn device_type() -> DeviceType {
        DeviceType::Unknown
    }
}