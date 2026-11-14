//! Upward channel: what the brain hears and when. Reports prediction errors, not state.

mod digest;
mod event;
mod habituation;
mod upward;

pub use digest::{estimate_tokens, Digest};
pub use event::{Event, EventClass};
pub use habituation::{Habituation, HABITUATED_BELOW, RECOVERY_TICKS};
pub use upward::{Upward, WAKE_MIN_GAP_TICKS};

/// DIGEST token budget, design note section 4.5.
pub const DEFAULT_DIGEST_TOKENS: usize = 150;
