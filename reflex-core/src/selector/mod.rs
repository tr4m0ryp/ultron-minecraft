//! Basal-ganglia style selection: priority class plus gain-weighted salience, a winner per
//! body channel, hysteresis, and commitment locks that only P0 breaks.

mod arbiter;
mod compose;
mod salience;

pub use arbiter::{Allocation, Candidate, Selector, HYSTERESIS};
pub use compose::{compose, resolve_gaze, MotorOutput};
pub use salience::{salience, Salience};
