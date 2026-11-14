//! Looking after the body on request: sleep through the night, eat right now.

mod eat;
mod sleep;

pub use eat::EatNow;
pub use sleep::{SleepInBed, BED_RADIUS};
