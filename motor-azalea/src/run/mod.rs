//! The per-tick loop: snapshot -> executor -> apply.

mod state;

pub use state::{handle, State};
