//! Replay protection for a single direction of traffic.
//!
//! TCP preserves byte order, so frames arrive in the exact order they were
//! sent. That makes strict monotonic counters the *correct* and simplest
//! replay policy: any counter that is not strictly greater than the highest
//! seen is a duplicate, a reorder, or an injected re-frame — all rejected.
//!
//! Because keys are derived fresh per connection (ephemeral X25519), a frame
//! captured from an old connection decrypts to garbage under the new keys and
//! never reaches this guard with a valid nonce. The guard is a second,
//! independent layer of defense.

/// Tracks the highest frame counter seen on one direction.
#[derive(Debug, Clone, Default)]
pub struct ReplayGuard {
    highest: u64,
}

impl ReplayGuard {
    /// A fresh guard that has never accepted a frame.
    pub fn new() -> Self {
        Self { highest: 0 }
    }

    /// Attempt to commit `counter`. Returns `true` if it is fresh (strictly
    /// greater than any previously accepted counter), `false` on replay.
    ///
    /// Counter `0` is reserved and never valid, so a connection that sends
    /// nothing can never be replayed into accepting frame zero.
    pub fn check_and_commit(&mut self, counter: u64) -> bool {
        if counter == 0 {
            return false;
        }
        if counter <= self.highest {
            return false;
        }
        self.highest = counter;
        true
    }

    /// The highest counter accepted so far (0 before the first frame).
    pub fn highest(&self) -> u64 {
        self.highest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_monotonic_sequence() {
        let mut g = ReplayGuard::new();
        assert!(g.check_and_commit(1));
        assert!(g.check_and_commit(2));
        assert!(g.check_and_commit(3));
        assert_eq!(g.highest(), 3);
    }

    #[test]
    fn rejects_replays() {
        let mut g = ReplayGuard::new();
        assert!(g.check_and_commit(10));
        assert!(!g.check_and_commit(10), "exact replay");
        assert!(!g.check_and_commit(5), "replay of an old counter");
        assert!(!g.check_and_commit(0), "reserved counter");
    }

    #[test]
    fn allows_large_jump_but_rejects_old_frames_after() {
        let mut g = ReplayGuard::new();
        assert!(g.check_and_commit(1_000_000));
        assert!(g.check_and_commit(1_000_001));
        assert!(!g.check_and_commit(999_999));
        assert!(!g.check_and_commit(1_000_000));
    }

    #[test]
    fn fresh_guard_rejects_first_nonpositive() {
        let mut g = ReplayGuard::new();
        assert!(!g.check_and_commit(0));
        assert!(g.check_and_commit(1));
    }

    #[test]
    fn clone_keeps_state() {
        let mut g = ReplayGuard::new();
        g.check_and_commit(7);
        let mut h = g.clone();
        assert!(!h.check_and_commit(7));
        assert!(h.check_and_commit(8));
    }
}
