//! Picking up what a dug block dropped when stepping into the gap did not: a log falling
//! off a trunk, a drop that rolled away. The drop is found as an entity and walked to with
//! the route planner, or straight at when no route exists.

use crate::behaviours::common::heading_to;
use crate::channels::{Commands, LegsCmd, Speed};
use crate::goals::gather::collect::straight_clear;
use crate::goals::ports::NavTarget;
use crate::goals::task::{GoalCx, Plan};
use crate::goals::travel::{Walk, Walker};
use crate::math::BlockPos;

/// Drops this far from the dug block count as its own.
const DROP_RADIUS: f64 = 4.0;
/// Ticks to wait for the drop entity to appear before moving on.
const SPAWN_WAIT: u32 = 10;
/// Ticks spent chasing one block's drop at most.
const PICKUP_TICKS: u32 = 120;
/// Closer than this, with a clear straight line, the drop is walked to directly.
const DIRECT: f64 = 6.0;

#[derive(Default)]
pub struct Pickup {
    ticks: u32,
    walker: Walker,
    /// The pathfinder found no way to this block's drop; walk straight at it instead.
    no_path: bool,
}

impl Pickup {
    /// Call once the block is broken.
    pub fn arm(&mut self) {
        self.ticks = 0;
        self.no_path = false;
        self.walker.reset();
    }

    pub fn reset_walk(&mut self) {
        self.walker.reset();
    }

    /// One tick of chasing the drop of the block dug at `pos`; `None` once done.
    pub fn step(&mut self, cx: &mut GoalCx, pos: BlockPos) -> Option<Plan> {
        if !cx.preempted {
            self.ticks += 1;
        }
        let me = cx.me_pos();
        let drops = cx.world.item_drops(None, pos.feet_centre(), DROP_RADIUS);
        let near = drops
            .iter()
            .min_by(|a, b| a.pos.dist(me).total_cmp(&b.pos.dist(me)));
        let give_up = self.ticks > PICKUP_TICKS || (near.is_none() && self.ticks > SPAWN_WAIT);
        let Some(d) = near.filter(|_| !give_up) else {
            return self.finish(cx);
        };
        if self.no_path || (d.pos.dist(me) <= DIRECT && straight_clear(cx.snap(), me, d.pos)) {
            self.walker.stop(cx);
            let legs = LegsCmd::toward(heading_to(me, d.pos), Speed::Walk);
            return Some(Plan::new("goal_pickup", Commands::default().legs(legs)));
        }
        let t = NavTarget::Block {
            pos: d.pos.block(),
            radius: 1.0,
        };
        match self.walker.walk(cx, t, "goal_pickup") {
            Walk::Moving(plan) => Some(plan),
            Walk::Arrived => {
                let legs = LegsCmd::toward(heading_to(me, d.pos), Speed::Walk);
                Some(Plan::new("goal_pickup", Commands::default().legs(legs)))
            }
            // No path (a drop on leaves, in a shallow hole): head straight at it for the
            // rest of the pickup time, as a player would, instead of giving up at once.
            Walk::Failed(_) => {
                self.no_path = true;
                let legs = LegsCmd::toward(heading_to(me, d.pos), Speed::Walk);
                Some(Plan::new("goal_pickup", Commands::default().legs(legs)))
            }
        }
    }

    fn finish(&mut self, cx: &mut GoalCx) -> Option<Plan> {
        self.walker.stop(cx);
        None
    }
}
