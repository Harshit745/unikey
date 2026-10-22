use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
)]pub enum Platform {
    Windows,
    MacOS,
    Linux,
    Android,
    IOS,
    Unknown,
}
use std::fmt;

impl fmt::Display for Platform {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Platform {
    pub fn current() -> Self {
        match std::env::consts::OS {
            "windows" => Self::Windows,
            "macos" => Self::MacOS,
            "linux" => Self::Linux,
            "android" => Self::Android,
            "ios" => Self::IOS,
            _ => Self::Unknown,
        }
    }

    /// Parse a platform name (as carried by discovery beacons / protocol
    /// payloads) back into a [`Platform`]. Accepts the `Display`/`Debug`
    /// spellings (`"MacOS"`, `"Windows"`, …) plus common lowercase/capitalised
    /// variants. `None` when unrecognised — callers fall back to `Unknown`.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Windows" | "windows" | "win32" => Some(Self::Windows),
            "MacOS" | "macOS" | "macos" | "Darwin" | "darwin" => Some(Self::MacOS),
            "Linux" | "linux" => Some(Self::Linux),
            "Android" | "android" => Some(Self::Android),
            "IOS" | "iOS" | "ios" => Some(Self::IOS),
            "Unknown" | "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_spellings() {
        assert_eq!(Platform::parse("Windows"), Some(Platform::Windows));
        assert_eq!(Platform::parse("MacOS"), Some(Platform::MacOS));
        assert_eq!(Platform::parse("macos"), Some(Platform::MacOS));
        assert_eq!(Platform::parse("Linux"), Some(Platform::Linux));
        assert_eq!(Platform::parse("IOS"), Some(Platform::IOS));
        assert_eq!(Platform::parse("Android"), Some(Platform::Android));
    }

    #[test]
    fn unknown_strings_yield_none() {
        assert_eq!(Platform::parse("Toaster"), None);
        assert_eq!(Platform::parse(""), None);
    }

    #[test]
    fn display_and_parse_round_trip() {
        for p in [
            Platform::Windows,
            Platform::MacOS,
            Platform::Linux,
            Platform::Android,
            Platform::IOS,
        ] {
            assert_eq!(Platform::parse(&p.to_string()), Some(p));
        }
    }
}