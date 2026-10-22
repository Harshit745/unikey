#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl Version {
    pub const fn new(
        major: u16,
        minor: u16,
        patch: u16,
    ) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

pub const UNIKEY_VERSION: Version =
    Version::new(0, 1, 0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_comparison_is_structural() {
        let a = Version::new(0, 1, 0);
        let b = Version::new(0, 1, 0);
        let c = Version::new(0, 1, 1);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn shipped_version_matches_its_components() {
        let v = UNIKEY_VERSION;
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 1);
        assert_eq!(v.patch, 0);
    }
}