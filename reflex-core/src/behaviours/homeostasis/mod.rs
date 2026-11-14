//! Level 5 homeostatic drives (hypothalamus analogue) and the companion leash.

mod companion;
mod eat;
mod health;
mod hunger;

pub use companion::CompanionLeash;
pub use eat::{best_food, eat_commands, EATING_TICKS};
pub use health::Health;
pub use hunger::Hunger;
