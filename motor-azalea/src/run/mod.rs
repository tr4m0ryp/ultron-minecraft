//! The per-tick loop: snapshot -> executor -> apply.

mod link;
mod state;

pub use link::{Link, BRIDGE_ENV};
pub use state::{handle, State};
