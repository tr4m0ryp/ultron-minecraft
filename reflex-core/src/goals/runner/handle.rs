use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::status::GoalStatus;
use crate::goals::ledger::Ledger;
use crate::knobs::Knobs;
use crate::trace::Probe;

#[derive(Debug, Default)]
struct Inner {
    finished: Cell<Option<u64>>,
    status: RefCell<Option<GoalStatus>>,
    ledger: Ledger,
    probe: Probe,
}

/// Shared between the goal runner and the adapter's tick loop. The runner records the id of
/// a goal that finished or failed; `settle` ends it in the knobs so the next queued goal
/// starts. Behaviours only read the knobs, so the loop that owns the executor does this.
/// The runner also publishes the latest goal status here for the brain to read.
#[derive(Clone, Debug, Default)]
pub struct GoalHandle(Rc<Inner>);

impl GoalHandle {
    /// A handle whose goals share `ledger`, the record of blocks the player placed.
    pub fn with_ledger(ledger: Ledger) -> Self {
        Self(Rc::new(Inner {
            ledger,
            ..Inner::default()
        }))
    }

    pub fn ledger(&self) -> &Ledger {
        &self.0.ledger
    }

    /// The flight-recorder probe the runner writes and the executor reads.
    pub fn probe(&self) -> Probe {
        self.0.probe.clone()
    }

    pub(super) fn finish(&self, goal_id: u64) {
        self.0.finished.set(Some(goal_id));
    }

    pub(super) fn publish(&self, status: GoalStatus) {
        *self.0.status.borrow_mut() = Some(status);
    }

    /// The latest status: of the active goal, or the outcome of the last one that ended.
    pub fn status(&self) -> Option<GoalStatus> {
        self.0.status.borrow().clone()
    }

    /// Ends the active goal if it is the one that finished. Returns whether it did.
    pub fn settle(&self, knobs: &mut Knobs) -> bool {
        let Some(id) = self.0.finished.take() else {
            return false;
        };
        if knobs.goal.as_ref().is_some_and(|g| g.id == id) {
            knobs.end_goal();
            true
        } else {
            false
        }
    }
}
