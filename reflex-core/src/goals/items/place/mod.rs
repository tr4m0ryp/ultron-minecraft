//! Placing one block. The skill gets itself into position: it stands where its eye sees a
//! face of a support within reach, on a short scaffold pillar when no floor spot works
//! (the route planner builds the pillar), places against that face, and takes its
//! scaffold down afterwards.

mod cleanup;
mod stand;

pub use cleanup::Cleanup;
pub use stand::Spot;

use super::equip::equip;
use super::surfaces::replaceable;
use crate::behaviours::common::away_from;
use crate::channels::{Commands, LegsCmd, MainCmd, Speed};
use crate::goals::ports::{kind_matches, NavTarget, Navigator};
use crate::goals::task::{GoalCx, GoalTask, Plan, Step};
use crate::goals::travel::route::SCAFFOLD;
use crate::goals::travel::{Walk, Walker};
use crate::math::BlockPos;
use stand::{eye_at, face_point, supports, visible_support};

/// Ticks between placement attempts, so the server's answer arrives before a retry.
const RETRY_GAP: u32 = 5;
const MAX_ATTEMPTS: u32 = 8;
/// Standing spots tried before the cell counts as out of reach.
const MAX_SPOTS: u32 = 3;

pub struct Place {
    block: String,
    pos: BlockPos,
    walker: Walker,
    attempts: u32,
    wait: u32,
    spot: Option<Spot>,
    spots_tried: u32,
    /// Cells a fill still has to place: never stand in them or pillar through them.
    keep_clear: Vec<BlockPos>,
    /// A lone place goal takes its own scaffold down; a fill does it once at the end.
    cleanup: Option<Cleanup>,
    outcome: Option<Step>,
}

impl Place {
    pub fn new(block: String, pos: BlockPos) -> Self {
        let mut p = Self::in_fill(block, pos, Vec::new());
        p.cleanup = Some(Cleanup::default());
        p
    }

    pub fn in_fill(block: String, pos: BlockPos, keep_clear: Vec<BlockPos>) -> Self {
        Self {
            block,
            pos,
            walker: Walker::default(),
            attempts: 0,
            wait: 0,
            spot: None,
            spots_tried: 0,
            keep_clear,
            cleanup: None,
            outcome: None,
        }
    }

    /// Scaffold blocks this placement's walks put down.
    pub fn take_scaffold(&mut self) -> Vec<BlockPos> {
        std::mem::take(&mut self.walker.placed)
    }

    fn at(&self) -> String {
        format!("{} {} {}", self.pos.x, self.pos.y, self.pos.z)
    }

    fn conclude(&mut self, cx: &mut GoalCx, step: Step) -> Step {
        let scaffold = self.take_scaffold();
        match self.cleanup.as_mut() {
            Some(c) if !scaffold.is_empty() => {
                c.add(scaffold);
                self.outcome = Some(step);
                self.step(cx)
            }
            _ => step,
        }
    }

    fn scaffold_held(cx: &GoalCx) -> u32 {
        SCAFFOLD.iter().map(|b| cx.world.count_item(b)).sum()
    }

    /// Walk to a spot that sees a face of a support.
    fn position(&mut self, cx: &mut GoalCx) -> Step {
        if self.spot.is_none() {
            if self.spots_tried >= MAX_SPOTS {
                return Step::Failed(format!("no spot found that sees a face at {}", self.at()));
            }
            let held = Self::scaffold_held(cx);
            self.spot = stand::find(cx, self.pos, &self.keep_clear, held);
            self.spots_tried += 1;
            if self.spot.is_none() {
                return Step::Failed(format!(
                    "nowhere to stand within reach of {} that sees a face to place against",
                    self.at()
                ));
            }
        }
        let spot = self.spot.expect("set above");
        eprintln!("DEBUG spot {spot:?} me {:?}", cx.me_pos());
        let t = NavTarget::Block {
            pos: spot.feet,
            radius: 0.5,
        };
        match self.walker.walk(cx, t, "goal_place_walk") {
            Walk::Moving(p) => Step::Bid(p),
            // Arrived but the face is still hidden (the world changed): pick another spot.
            Walk::Arrived => {
                self.spot = None;
                Step::Bid(Plan::hold("goal_place"))
            }
            Walk::Failed(_) => {
                self.spot = None;
                Step::Bid(Plan::hold("goal_place"))
            }
        }
    }
}

impl GoalTask for Place {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        if let Some(out) = self.outcome.clone() {
            let c = self.cleanup.as_mut().expect("outcome waits only for a cleanup");
            if let Some(p) = c.step(cx) {
                return Step::Bid(p);
            }
            return out;
        }
        let current = cx.world.block_at(self.pos);
        if current.as_deref().is_some_and(|n| kind_matches(&self.block, n)) {
            let done = Step::Done(format!("placed {} at {}", self.block, self.at()));
            return self.conclude(cx, done);
        }
        if let Some(n) = current.as_deref().filter(|n| !replaceable(n)) {
            let failed = Step::Failed(format!("{} is occupied by {n}", self.at()));
            return self.conclude(cx, failed);
        }
        let Some(slot) = cx.world.item_slot(&self.block) else {
            return self.conclude(cx, Step::Failed(format!("no {} to place", self.block)));
        };
        let sup = supports(cx, self.pos);
        if sup.is_empty() {
            let why = match stand::opener_beside(cx, self.pos) {
                Some(k) => format!(
                    "only a {k} to place against at {}; clicking it would open it",
                    self.at()
                ),
                None => format!("nothing to place against at {}", self.at()),
            };
            return self.conclude(cx, Step::Failed(why));
        }
        let me = cx.snap().me.clone();
        let feet = me.pos.block();
        if feet == self.pos || feet.above() == self.pos {
            cx.blocked("standing in the spot to fill");
            let legs = LegsCmd::toward(away_from(me.pos, self.pos.feet_centre()), Speed::Walk);
            return Step::Bid(Plan::new("goal_place", Commands::default().legs(legs)));
        }
        let eye = if me.on_ground { me.eye() } else { eye_at(feet) };
        let Some(support) = visible_support(eye, self.pos, &sup) else {
            return self.position(cx);
        };
        self.walker.stop(cx);
        if let Some(c) = equip(&me, slot) {
            return Step::Bid(Plan::new("goal_equip", c));
        }
        let aim = Commands::default()
            .legs(LegsCmd::stop())
            .look_at(face_point(self.pos, support));
        if self.wait > 0 {
            if !cx.preempted {
                self.wait -= 1;
            }
            return Step::Bid(Plan::new("goal_place", aim));
        }
        if self.attempts >= MAX_ATTEMPTS {
            let failed = Step::Failed(format!("the server did not accept a block at {}", self.at()));
            return self.conclude(cx, failed);
        }
        if !cx.preempted {
            self.attempts += 1;
            self.wait = RETRY_GAP;
        }
        let click = MainCmd::PlaceAt {
            cell: self.pos,
            against: support,
        };
        Step::Bid(Plan::new("goal_place", aim.main(click)))
    }

    fn stop(&mut self, _nav: &mut dyn Navigator) {
        self.walker.reset();
        self.spot = None;
        if let Some(c) = self.cleanup.as_mut() {
            c.reset_walk();
        }
    }
}
