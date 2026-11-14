//! A stand-in executor: real `Knobs` and `Upward`, a hand-built `Snapshot`, and a toy
//! world that walks toward the goal so the brain loop can be exercised without the game.

mod world;

pub use world::FakeWorld;

use reflex_core::events::Upward;
use reflex_core::knobs::Knobs;

use crate::host::Host;

pub struct FakeExecutor {
    pub knobs: Knobs,
    pub upward: Upward,
    pub legs_owner: Option<String>,
}

impl Default for FakeExecutor {
    fn default() -> Self {
        Self {
            knobs: Knobs::new(),
            upward: Upward::default(),
            legs_owner: None,
        }
    }
}

impl Host for FakeExecutor {
    fn knobs(&mut self) -> &mut Knobs {
        &mut self.knobs
    }
    fn upward(&mut self) -> &mut Upward {
        &mut self.upward
    }
    fn legs_owner(&self) -> Option<String> {
        self.legs_owner.clone()
    }
}
