use super::walker::{Walk, Walker};
use crate::goals::ports::{NavTarget, Navigator};
use crate::goals::task::{GoalCx, GoalTask, Step};

pub const COME_DISTANCE: f64 = 2.5;
/// Ticks the companion may be out of view before the goal fails.
const LOST_LIMIT: u32 = 100;

pub struct ComeHere {
    walker: Walker,
    lost: u32,
}

impl ComeHere {
    pub fn new() -> Self {
        Self {
            walker: Walker::default(),
            lost: 0,
        }
    }
}

impl Default for ComeHere {
    fn default() -> Self {
        Self::new()
    }
}

impl GoalTask for ComeHere {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        let Some(id) = cx.snap().companion.as_ref().and_then(|c| c.entity_id) else {
            self.lost += 1;
            if self.lost > LOST_LIMIT {
                return Step::Failed("companion not visible".into());
            }
            cx.stuck("companion not visible");
            return Step::Idle;
        };
        self.lost = 0;
        let target = NavTarget::Entity {
            id,
            distance: COME_DISTANCE,
        };
        match self.walker.walk(cx, target, "goal_come_here") {
            Walk::Arrived => Step::Done("next to the companion".into()),
            Walk::Moving(plan) => Step::Bid(plan),
            Walk::Failed(f) => Step::Failed(format!("cannot reach the companion: {f:?}")),
        }
    }

    fn stop(&mut self, _nav: &mut dyn Navigator) {
        self.walker.reset();
    }
}
