pub mod event;
pub mod platform;
pub mod service;
pub mod cursor;
mod capture;
mod outbound;
#[cfg(windows)]
mod windows_cursor;

pub use service::InputService;
