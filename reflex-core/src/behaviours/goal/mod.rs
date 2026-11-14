//! The brain's active goal as one P5 behaviour whose urgency is the priority the brain set.

use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::behaviours::common::heading_to;
use crate::channels::{Commands, LegsCmd, Speed};
use crate::events::EventClass;

pub const GOAL_ID: &str = "GOAL";
const ARRIVED: f64 = 1.0;
/// Gait choice (part of C1): sprint on long legs when food allows sprinting.
const SPRINT_BEYOND: f64 = 8.0;

/// Walks straight toward the goal's target. Path planning is out of scope; goals without a
/// target do not bid.
#[derive(Default)]
pub struct GoalExecutor {
    arrived_reported: Option<u64>,
}

impl Behaviour for GoalExecutor {
    fn id(&self) -> &'static str {
        GOAL_ID
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let goal = ctx.knobs.goal.as_ref()?;
        let target = goal.target?;
        let me = &ctx.snap.me;
        let d = (target - me.pos).horizontal().len();
        if d <= ARRIVED {
            if self.arrived_reported != Some(goal.id) {
                self.arrived_reported = Some(goal.id);
                ctx.emit(ctx.report(EventClass::Wake, "goal_reached", "").text(goal.spec.clone()));
            }
            return None;
        }
        let speed = if d > SPRINT_BEYOND && me.food > 6 { Speed::Sprint } else { Speed::Walk };
        let legs = LegsCmd::toward(heading_to(me.pos, target), speed);
        Some(Bid::new(Priority::P5, goal.priority.max(0.05), "goal", Commands::default().legs(legs)))
    }
}
