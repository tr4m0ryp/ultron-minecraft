//! Text interface to the brain: commands in on stdin, upward events and an optional
//! per-tick trace out on stdout.

mod inbox;
mod outbox;
mod trace;

pub use inbox::{BrainCmd, Inbox};
pub use outbox::{dump_trace, publish};
pub use trace::Trace;
