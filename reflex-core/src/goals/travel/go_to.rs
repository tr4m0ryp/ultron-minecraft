use super::walker::{Walk, Walker};
use crate::goals::ports::NavTarget;
use crate::goals::task::{GoalCx, GoalTask, Step};
use crate::math::Vec3;

pub const ARRIVE_RADIUS: f64 = 1.0;

pub struct GoTo {
    pos: Vec3,
    walker: Walker,
    start_dist: Option<f64>,
}

impl GoTo {
    pub fn new(pos: Vec3) -> Self {
        Self {
            pos,
            walker: Walker::default(),
            start_dist: None,
        }
    }
}

impl GoalTask for GoTo {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        let target = NavTarget::Block {
            pos: self.pos.block(),
            radius: ARRIVE_RADIUS,
        };
        let d = cx.me_pos().dist(self.pos);
        let start = *self.start_dist.get_or_insert(d);
        match self.walker.walk(cx, target, "goal_go_to") {
            Walk::Arrived => Step::Done(format!(
                "arrived at {:.0} {:.0} {:.0}",
                self.pos.x, self.pos.y, self.pos.z
            )),
            Walk::Moving(plan) => {
                let left = (start - d).max(0.0) as u32;
                cx.progress(left, start as u32, format!("{d:.0} blocks to go"));
                Step::Bid(plan)
            }
            Walk::Failed(f) => Step::Failed(format!(
                "cannot reach {:.0} {:.0} {:.0}: {f:?}",
                self.pos.x, self.pos.y, self.pos.z
            )),
        }
    }

    fn stop(&mut self, _nav: &mut dyn crate::goals::ports::Navigator) {
        self.walker.reset();
    }
}
