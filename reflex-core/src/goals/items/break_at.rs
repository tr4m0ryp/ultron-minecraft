//! Breaking one named block: the explicit removal. Everything else the body does leaves the
//! blocks it placed (and protected builds) alone; this goal digs the cell it is told to,
//! even one the player placed.

use super::dig_out::dig_out;
use super::surfaces::replaceable;
use crate::goals::ports::Navigator;
use crate::goals::task::{GoalCx, GoalTask, Step};
use crate::goals::travel::Walker;
use crate::math::BlockPos;

pub struct BreakAt {
    pos: BlockPos,
    walker: Walker,
    ticks: u32,
    dug: bool,
}

impl BreakAt {
    pub fn new(pos: BlockPos) -> Self {
        Self {
            pos,
            walker: Walker::default(),
            ticks: 0,
            dug: false,
        }
    }

    fn at(&self) -> String {
        format!("{} {} {}", self.pos.x, self.pos.y, self.pos.z)
    }
}

impl GoalTask for BreakAt {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        let Some(held) = cx.world.block_at(self.pos) else {
            return Step::Failed(format!("{} is not loaded", self.at()));
        };
        if replaceable(&held) {
            cx.ledger.forget(self.pos);
            return Step::Done(if self.dug {
                format!("broke the block at {}", self.at())
            } else {
                format!("nothing to break at {}: it holds {held}", self.at())
            });
        }
        self.dug = true;
        match dig_out(
            cx,
            &mut self.walker,
            &mut self.ticks,
            self.pos,
            &held,
            "goal_break",
        ) {
            Ok(plan) => Step::Bid(plan),
            Err(why) => Step::Failed(why),
        }
    }

    fn stop(&mut self, _nav: &mut dyn Navigator) {
        self.walker.reset();
    }
}
