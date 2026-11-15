//! Always-on flight recorder: the last few hundred ticks of selector decisions and goal
//! state in a fixed ring, rendered run-length encoded when a goal stalls or fails so the next
//! stall can be diagnosed from the log instead of guessed at.

mod goal;
mod probe;
mod render;
mod ring;

pub use goal::{GoalTick, NavCode};
pub use probe::Probe;
pub use ring::{Decision, TickRec, TickRing, RING_TICKS, VETO_SLOTS};
