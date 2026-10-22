use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use rand_core::{OsRng, RngCore};
use tokio::sync::Mutex;
use uuid::Uuid;

/// The current phase of a pairing exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingState {
    /// Code/QR visible on-screen, waiting for the remote device to respond.
    Pending,
    /// Remote device sent a valid pairing request; handshake in progress.
    Accepted(Uuid),
    /// Pairing token has expired without completion.
    Expired,
    /// Both devices have exchanged identities and saved each other to the trust store.
    Completed,
}

/// A single pairing session, created when the local user opens the Pair screen.
#[derive(Debug, Clone)]
pub struct PairingSession {
    /// Opaque identifier for this session (used in QR data and API responses).
    pub session_id: Uuid,
    /// Six-digit numeric code the user can read aloud or type.
    pub pair_code: String,
    /// URI-encoded data string for QR-code rendering.
    /// Format: `unikey://pair/{device_id}/{session_id}/{code}`
    pub qr_data: String,
    /// Monotonic clock instant when this session was created.
    pub created_at: Instant,
    /// Deadline after which the token is invalid (default 60 s).
    pub expires_at: Instant,
    /// Current state in the pairing state machine.
    pub state: PairingState,
    /// The device_id of the local device that created this session.
    pub local_device_id: Uuid,
}

impl PairingSession {
    /// Returns true if this session has passed its expiry deadline.
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    /// Returns the number of seconds until this session expires.
    pub fn remaining_seconds(&self) -> u64 {
        self.expires_at
            .checked_duration_since(Instant::now())
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

/// Thread-safe manager for active pairing sessions.
///
/// Sessions auto-expire after a configurable TTL (default 60 s).
/// Expired sessions are removed by a background cleanup task.
#[derive(Debug, Clone)]
pub struct PairingSessionManager {
    sessions: Arc<Mutex<HashMap<Uuid, PairingSession>>>,
    /// Alternative index: pair_code → session_id for O(1) lookup during validation.
    code_index: Arc<Mutex<HashMap<String, Uuid>>>,
}

impl Default for PairingSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PairingSessionManager {
    /// Create an empty session manager.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            code_index: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Generate a new 6-digit pairing code string, zero-padded.
    pub fn generate_pair_code() -> String {
        let mut rng = OsRng;
        let val: u32 = rng.next_u32() % 1_000_000;
        format!("{val:06}")
    }

    /// Build the QR-code URI payload for a session.
    pub fn build_qr_data(device_id: Uuid, session_id: Uuid, pair_code: &str) -> String {
        format!("unikey://pair/{device_id}/{session_id}/{pair_code}")
    }

    /// Create a fresh pairing session with a random code and QR data.
    ///
    /// Returns the session, which the caller should return to the Flutter UI.
    pub async fn create_session(&self, device_id: Uuid, ttl_secs: u64) -> PairingSession {
        let now = Instant::now();
        let session_id = Uuid::new_v4();
        let pair_code = Self::generate_pair_code();
        let qr_data = Self::build_qr_data(device_id, session_id, &pair_code);

        let session = PairingSession {
            session_id,
            pair_code: pair_code.clone(),
            qr_data,
            created_at: now,
            expires_at: now + std::time::Duration::from_secs(ttl_secs),
            state: PairingState::Pending,
            local_device_id: device_id,
        };

        {
            let mut sessions = self.sessions.lock().await;
            sessions.insert(session_id, session.clone());
        }
        {
            let mut index = self.code_index.lock().await;
            index.insert(pair_code, session_id);
        }

        session
    }

    /// Retrieve a session by its session_id.
    pub async fn get_session(&self, session_id: &Uuid) -> Option<PairingSession> {
        let sessions = self.sessions.lock().await;
        sessions.get(session_id).cloned()
    }

    /// Look up a session by its 6-digit pair code (used during PairRequest validation).
    pub async fn find_by_code(&self, code: &str) -> Option<PairingSession> {
        let index = self.code_index.lock().await;
        let session_id = index.get(code)?;
        let sessions = self.sessions.lock().await;
        sessions.get(session_id).cloned()
    }

    /// Advance the state of a session.
    pub async fn update_state(&self, session_id: &Uuid, state: PairingState) {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.state = state;
        }
    }

    /// Remove a session and its code-index entry. Called after completion or expiry.
    pub async fn remove_session(&self, session_id: &Uuid) {
        let mut index = self.code_index.lock().await;
        let mut sessions = self.sessions.lock().await;

        if let Some(session) = sessions.remove(session_id) {
            index.remove(&session.pair_code);
        }
    }

    /// Remove all expired sessions. Returns the count removed.
    pub async fn cleanup_expired(&self) -> usize {
        let mut sessions = self.sessions.lock().await;
        let mut index = self.code_index.lock().await;

        let expired_ids: Vec<Uuid> = sessions
            .iter()
            .filter(|(_, s)| s.is_expired() && s.state != PairingState::Completed)
            .map(|(id, _)| *id)
            .collect();

        let count = expired_ids.len();
        for id in &expired_ids {
            if let Some(s) = sessions.remove(id) {
                index.remove(&s.pair_code);
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_code_is_six_zero_padded_digits() {
        for _ in 0..200 {
            let code = PairingSessionManager::generate_pair_code();
            assert_eq!(code.len(), 6, "code {code} must be exactly 6 digits");
            assert!(code.chars().all(|c| c.is_ascii_digit()), "code {code} must be numeric");
        }
    }

    #[test]
    fn qr_data_encodes_the_expected_uri() {
        let device = Uuid::new_v4();
        let session = Uuid::new_v4();
        let uri = PairingSessionManager::build_qr_data(device, session, "123456");
        assert_eq!(uri, format!("unikey://pair/{device}/{session}/123456"));
    }

    #[tokio::test]
    async fn created_session_is_lookupable_by_code_and_id() {
        let mgr = PairingSessionManager::new();
        let device = Uuid::new_v4();
        let session = mgr.create_session(device, 60).await;

        assert_eq!(session.local_device_id, device);
        assert_eq!(session.state, PairingState::Pending);

        // Code index lookup.
        let by_code = mgr.find_by_code(&session.pair_code).await.expect("code resolves");
        assert_eq!(by_code.session_id, session.session_id);

        // Session id lookup.
        let by_id = mgr.get_session(&session.session_id).await.expect("id resolves");
        assert_eq!(by_id.session_id, session.session_id);

        // Unknown code/id resolve to None.
        assert!(mgr.find_by_code("000000").await.is_none());
        assert!(mgr.get_session(&Uuid::new_v4()).await.is_none());
    }

    #[tokio::test]
    async fn sessions_get_unique_codes() {
        let mgr = PairingSessionManager::new();
        let device = Uuid::new_v4();
        let a = mgr.create_session(device, 60).await;
        let b = mgr.create_session(device, 60).await;
        assert_ne!(a.pair_code, b.pair_code);
        assert_ne!(a.session_id, b.session_id);
    }

    #[tokio::test]
    async fn session_advances_through_states() {
        let mgr = PairingSessionManager::new();
        let device = Uuid::new_v4();
        let session = mgr.create_session(device, 60).await;
        let id = session.session_id;
        let peer = Uuid::new_v4();

        mgr.update_state(&id, PairingState::Accepted(peer)).await;
        let s = mgr.get_session(&id).await.unwrap();
        assert_eq!(s.state, PairingState::Accepted(peer));

        mgr.update_state(&id, PairingState::Completed).await;
        let s = mgr.get_session(&id).await.unwrap();
        assert_eq!(s.state, PairingState::Completed);
    }

    #[tokio::test]
    async fn expired_sessions_are_cleaned_but_completed_ones_survive() {
        let mgr = PairingSessionManager::new();
        let device = Uuid::new_v4();

        // A session already past its TTL.
        let expired = mgr.create_session(device, 1).await;
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // A completed session from the same TTL that must be kept.
        let completed = mgr.create_session(device, 1).await;
        mgr.update_state(&completed.session_id, PairingState::Completed).await;

        let removed = mgr.cleanup_expired().await;
        assert_eq!(removed, 1);
        assert!(mgr.get_session(&expired.session_id).await.is_none());
        assert!(mgr.get_session(&completed.session_id).await.is_some());

        // The expired code is gone from the index too.
        assert!(mgr.find_by_code(&expired.pair_code).await.is_none());
    }

    #[tokio::test]
    async fn remove_session_clears_both_indexes() {
        let mgr = PairingSessionManager::new();
        let session = mgr.create_session(Uuid::new_v4(), 60).await;
        mgr.remove_session(&session.session_id).await;
        assert!(mgr.get_session(&session.session_id).await.is_none());
        assert!(mgr.find_by_code(&session.pair_code).await.is_none());
    }
}
