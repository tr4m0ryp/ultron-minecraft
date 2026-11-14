use std::cell::Cell;
use std::rc::Rc;

use crate::knobs::Knobs;

/// Shared between the goal runner and the adapter's tick loop. The runner records the id of
/// a goal that finished or failed; `settle` ends it in the knobs so the next queued goal
/// starts. Behaviours only read the knobs, so the loop that owns the executor does this.
#[derive(Clone, Debug, Default)]
pub struct GoalHandle(Rc<Cell<Option<u64>>>);

impl GoalHandle {
    pub(super) fn finish(&self, goal_id: u64) {
        self.0.set(Some(goal_id));
    }

    /// Ends the active goal if it is the one that finished. Returns whether it did.
    pub fn settle(&self, knobs: &mut Knobs) -> bool {
        let Some(id) = self.0.take() else {
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
