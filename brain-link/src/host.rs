use reflex_core::events::Upward;
use reflex_core::knobs::Knobs;
use reflex_core::{Channel, Executor};

/// What the bridge needs from an executor: its knobs, its upward channel, and who holds
/// the legs. `Executor` implements it; tests use `fake::FakeExecutor`.
pub trait Host {
    fn knobs(&mut self) -> &mut Knobs;
    fn upward(&mut self) -> &mut Upward;
    fn legs_owner(&self) -> Option<String>;
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
