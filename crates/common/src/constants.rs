pub const DEFAULT_TCP_PORT: u16 = 45892;

pub const DEFAULT_DISCOVERY_PORT: u16 = 45893;

pub const PROTOCOL_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ports_are_distinct_and_in_range() {
        let mut ports = [DEFAULT_TCP_PORT, DEFAULT_DISCOVERY_PORT];
        ports.sort_unstable();
        assert_ne!(ports[0], ports[1], "TCP and discovery ports must differ");
        for p in ports {
            assert!(p > 0, "ports must be non-zero");
            assert!(p < u16::MAX, "ports must not collide with the reserved top");
        }
    }

    // (No test for PROTOCOL_VERSION itself: it is a compile-time constant, so
    // any assertion would be provably constant — the port invariants above are
    // the ones worth guarding.)
}