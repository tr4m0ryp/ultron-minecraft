//! The decision log: one JSON line per decision that arrives from thought, with the state
//! of the body in the tick the decision was applied. It is the training data for the
//! intuition layer. The first training run used pairs cut from session transcripts, whose
//! state was the last `observe` thought had seen (often thousands of ticks old) and held
//! neither the inventory nor the plan, so the next craft or mine step could not be
//! predicted by anything. Here the body writes the record itself, from this tick's
//! snapshot, before the decision changes anything.

mod record;
mod sink;
mod state;
#[cfg(test)]
mod tests;

pub use record::{Outcome, Record, FORMAT};
pub use sink::{default_path, Cost, Sink, DECISIONS_ENV};
pub use state::{EntityView, EventLine, GoalLine, Goals, State, EVENTS_KEPT, NEAR_MAX};

use crate::wire::Request;

/// A request that changes what the body does: every such call is a decision of thought.
pub fn is_decision(req: &Request) -> bool {
    matches!(
        req,
        Request::SetGoal { .. }
            | Request::SetMode { .. }
            | Request::SetGain { .. }
            | Request::Inhibit { .. }
            | Request::Arm { .. }
            | Request::Say { .. }
            | Request::Protect { .. }
    )
}
