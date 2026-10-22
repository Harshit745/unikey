use std::fmt;

/// Defines which roadmap phase a service belongs to.
/// Enables the daemon to group, log, and potentially gate services by phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    Phase1,
    Phase2,
    Phase3,
    Phase4,
    Phase5,
    Phase6,
}

impl Phase {
    pub fn label(&self) -> &'static str {
        match self {
            Phase::Phase1 => "Phase 1 — Core Connectivity & MVP",
            Phase::Phase2 => "Phase 2 — Productivity",
            Phase::Phase3 => "Phase 3 — Universal Control",
            Phase::Phase4 => "Phase 4 — Collaboration",
            Phase::Phase5 => "Phase 5 — AI Operating Layer",
            Phase::Phase6 => "Phase 6 — Enterprise & Fleet",
        }
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}
