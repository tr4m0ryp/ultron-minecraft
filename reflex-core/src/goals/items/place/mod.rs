//! Placing one block. The skill gets itself into position: it stands where its eye sees a
//! face of a support within reach, on a short scaffold pillar when no floor spot works
//! (the route planner builds the pillar), places against that face, and takes its
//! scaffold down afterwards.

mod approach;
mod cleanup;
mod clear;
mod footprint;
mod in_way;
mod stand;

pub use cleanup::Cleanup;
pub use clear::{clearance, Clearance};
pub use stand::Spot;
pub(crate) use stand::{eye_at, free, onto_centre};

use super::equip::equip;
use super::surfaces::replaceable;
use crate::channels::{Commands, LegsCmd, MainCmd};
use crate::goals::ports::{kind_matches, Navigator};
use crate::goals::task::{GoalCx, GoalTask, Plan, Step};
use crate::goals::travel::Walker;
use crate::math::BlockPos;
use footprint::{two_cells, AIM_SETTLE};
use stand::{face_point, supports, visible_support, CLICK_MARGIN};

/// Ticks between placement attempts, so the server's answer arrives before a retry.
const RETRY_GAP: u32 = 5;
const MAX_ATTEMPTS: u32 = 8;

pub struct Place {
    block: String,
    pos: BlockPos,
    walker: Walker,
    attempts: u32,
    wait: u32,
    /// Ticks the current aim has been held (two-cell blocks click only once it settles).
    aimed: u32,
    spot: Option<Spot>,
    spots_tried: u32,
    /// Ticks spent stepping onto the middle of the current spot.
    settle: u32,
    /// Spots the face was hidden from after all, or that could not be reached.
    tried: Vec<BlockPos>,
    in_way: in_way::InWay,
    /// Scaffold earlier placements of the same fill left standing.
    scaffold: Vec<BlockPos>,
    /// Cells a fill still has to place: never stand in them or pillar through them.
    keep_clear: Vec<BlockPos>,
    /// Every cell of the fill this placement belongs to, placed or not: a spot under one
    /// of them is a last resort (see `stand::UNDER_FILL_COST`).
    cover: Vec<BlockPos>,
    /// The goal said `replace`: clear a placed block or a protected cell too.
    replace: bool,
    clear_ticks: u32,
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
            aimed: 0,
            spot: None,
            spots_tried: 0,
            settle: 0,
            tried: Vec::new(),
            in_way: in_way::InWay::default(),
            scaffold: Vec::new(),
            keep_clear,
            cover: Vec::new(),
            replace: false,
            clear_ticks: 0,
            cleanup: None,
            outcome: None,
        }
    }

    /// `replace` lets the placement clear whatever is in the cell, placed or protected.
    pub fn replacing(mut self, replace: bool) -> Self {
        self.replace = replace;
        self
    }

    /// The whole fill this cell is part of: the body keeps out from under it.
    pub fn under(mut self, cells: &[BlockPos]) -> Self {
        self.cover = cells.to_vec();
        self
    }

    /// Scaffold still standing from earlier placements of the same fill.
    pub fn over_scaffold(mut self, blocks: &[BlockPos]) -> Self {
        self.scaffold = blocks.to_vec();
        self
    }

    /// Scaffold blocks this placement's walks put down.
    pub fn take_scaffold(&mut self) -> Vec<BlockPos> {
        std::mem::take(&mut self.walker.placed)
    }

    fn at(&self) -> String {
        format!("{} {} {}", self.pos.x, self.pos.y, self.pos.z)
    }

    /// A lone placement takes its scaffold down before it reports. In a fill the scaffold
    /// stays listed for the fill to collect: live, it was taken here and dropped, so the fill
    /// never took it down and the next cell did not know the block under its feet.
    fn conclude(&mut self, cx: &mut GoalCx, step: Step) -> Step {
        let Some(c) = self.cleanup.as_mut() else {
            return step;
        };
        if self.walker.placed.is_empty() {
            return step;
        }
        c.add(std::mem::take(&mut self.walker.placed));
        self.outcome = Some(step);
        self.step(cx)
    }
}

impl GoalTask for Place {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        if let Some(out) = self.outcome.clone() {
            let c = self
                .cleanup
                .as_mut()
                .expect("outcome waits only for a cleanup");
            if let Some(p) = c.step(cx) {
                return Step::Bid(p);
            }
            return out;
        }
        let current = cx.world.block_at(self.pos);
        if let Some(n) = current.as_deref().filter(|n| kind_matches(&self.block, n)) {
            // Seen standing after this goal's own click: the server took it, so it is the
            // player's own block from now on (the ledger).
            if self.attempts > 0 {
                cx.ledger.remember(self.pos, n);
            }
            let done = Step::Done(format!("placed {} at {}", self.block, self.at()));
            return self.conclude(cx, done);
        }
        if let Some(n) = current.as_deref().filter(|n| !replaceable(n)) {
            return match clearance(cx, self.pos, n, self.replace) {
                Clearance::Refuse(why) => self.conclude(cx, Step::Failed(why)),
                _ => self.clear(cx, n),
            };
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
        if stand::body_overlaps(me.pos, self.pos) {
            let (mine, earlier) = (&self.walker.placed, &self.scaffold);
            let scaffold = |b| mine.contains(&b) || earlier.contains(&b);
            return Step::Bid(self.in_way.step(cx, self.pos, scaffold));
        }
        self.in_way.reset();
        let eye = if me.on_ground { me.eye() } else { eye_at(feet) };
        let Some(support) = visible_support(cx, &self.block, eye, self.pos, &sup, CLICK_MARGIN)
        else {
            self.aimed = 0;
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
        // A bed's head follows the yaw the server holds at the click: turn first.
        if two_cells(&self.block) && self.aimed < AIM_SETTLE {
            if !cx.preempted {
                self.aimed += 1;
            }
            return Step::Bid(Plan::new("goal_place", aim));
        }
        if self.attempts >= MAX_ATTEMPTS {
            let failed = Step::Failed(format!(
                "the server did not accept a block at {}",
                self.at()
            ));
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
