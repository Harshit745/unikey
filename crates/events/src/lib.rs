//! Live event system: a pub/sub `EventBus` that services publish to and the
//! desktop GUI subscribes to over Server-Sent Events.
//!
//! - [`AppEvent`] — the typed set of events (clipboard sync, transfer
//!   progress, sessions, pairing, trust-store changes).
//! - [`EventBus`] — a shared broadcaster with a bounded in-memory history.
//! - [`Envelope`] — an event stamped with its publish time, serialized over
//!   the SSE stream.

pub mod bus;
pub mod event;

pub use bus::EventBus;
pub use event::{AppEvent, Envelope};
