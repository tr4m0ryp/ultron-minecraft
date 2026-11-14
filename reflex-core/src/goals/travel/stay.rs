use crate::behaviours::common::heading_to;
use crate::channels::{Commands, LegsCmd, Speed};
use crate::goals::task::{GoalCx, GoalTask, Plan, Step};
use crate::math::Vec3;

/// Pushed (or pulled by a reflex) further than this from the anchor, walk back.
const DRIFT: f64 = 1.5;

/// Open-ended: hold the spot where the goal started.
#[derive(Default)]
pub struct Stay {
    anchor: Option<Vec3>,
}

impl GoalTask for Stay {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        let me = cx.me_pos();
        let anchor = *self.anchor.get_or_insert(me);
        if (anchor - me).horizontal().len() > DRIFT {
            let legs = LegsCmd::toward(heading_to(me, anchor), Speed::Walk);
            return Step::Bid(Plan::new("goal_stay", Commands::default().legs(legs)));
        }
        Step::Bid(Plan::hold("goal_stay"))
    }
}
