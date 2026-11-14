//! Level 3 locomotion pattern generators (CPG analogue).

mod swim;
mod unstuck;

pub use swim::Swim;
pub use unstuck::{Unstuck, STUCK_TICKS, WAKE_AFTER_TICKS};
