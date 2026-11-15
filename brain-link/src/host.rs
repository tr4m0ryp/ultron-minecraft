use reflex_core::events::Upward;
use reflex_core::goals::GoalStatus;
use reflex_core::knobs::Knobs;
use reflex_core::{Channel, Executor};

/// What the bridge needs from an executor: its knobs, its upward channel, and who holds
/// the legs, plus the goal layer's status. `Executor` implements it (without goal status); tests use `fake::FakeExecutor`.
pub trait Host {
    fn knobs(&mut self) -> &mut Knobs;
    fn upward(&mut self) -> &mut Upward;
    fn legs_owner(&self) -> Option<String>;
    /// The goal layer's latest status, when the host runs one.
    fn goal_status(&self) -> Option<GoalStatus> {
        None
    }
    /// Main inventory and hotbar by exact item name, counts summed per name, for the
    /// decision log. Hosts without an inventory report none.
    fn inventory(&self) -> Vec<(String, u32)> {
        Vec::new()
    }
}

impl Host for Executor {
    fn knobs(&mut self) -> &mut Knobs {
        &mut self.knobs
    }
    fn upward(&mut self) -> &mut Upward {
        &mut self.upward
    }
    fn legs_owner(&self) -> Option<String> {
        self.owner(Channel::Legs).map(str::to_string)
    }
}
