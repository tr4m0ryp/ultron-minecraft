//! Mining the way a player mines: stand next to the block and level with it, break it,
//! step into the gap (breaking the block above too, so the way stays two high) and walk
//! over the drop. Blocks lower down are reached by a staircase the route planner digs,
//! never by digging down from above. Progress counts what reached the inventory: live, the
//! old goal mined stone 4 to 5 blocks below its feet and picked up 11 of 81 drops.

mod pickup;
mod target;

pub use target::SEARCH_RADIUS;

use crate::behaviours::common::heading_to;
use crate::channels::{Commands, LegsCmd, MainCmd, Speed};
use crate::goals::items::{equip, replaceable};
use crate::goals::ports::{kind_matches, NavTarget, Navigator, ToolNeed};
use crate::goals::task::{GoalCx, GoalTask, Plan, Step};
use crate::goals::travel::route::{drop_of, natural};
use crate::goals::travel::{Walk, Walker};
use crate::math::{BlockPos, Vec3};
use pickup::Pickup;

/// Digging one block for longer than this means something (a veto, a wrong tool) stops it.
pub const DIG_LIMIT_TICKS: u32 = 600;
/// Blocks given up on (unreachable or unbreakable) before the goal fails.
const MAX_SKIPPED: usize = 5;
/// Ticks to step into a dug cell before going on without it.
const ADVANCE_TICKS: u32 = 30;
/// Blocks broken beyond the count without their drops arriving before the goal fails.
const LOSS_SLACK: u32 = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Find,
    Approach(BlockPos),
    Dig { pos: BlockPos, ticks: u32 },
    Advance { pos: BlockPos, ticks: u32 },
    Pickup(BlockPos),
}

pub struct Mine {
    block: String,
    drop: String,
    count: u32,
    /// Drop items held when the goal began; progress is what was added since.
    held_at_start: Option<u32>,
    broken: u32,
    phase: Phase,
    walker: Walker,
    skipped: Vec<BlockPos>,
    pickup: Pickup,
}

impl Mine {
    pub fn new(block: String, count: u32) -> Self {
        Self {
            drop: drop_of(&block).to_string(),
            block,
            count: count.max(1),
            held_at_start: None,
            broken: 0,
            phase: Phase::Find,
            walker: Walker::default(),
            skipped: Vec::new(),
            pickup: Pickup::default(),
        }
    }

    fn gained(&self, cx: &GoalCx) -> u32 {
        let start = self.held_at_start.unwrap_or(0);
        cx.world.count_item(&self.drop).saturating_sub(start)
    }

    fn tally(&self, cx: &GoalCx) -> String {
        format!(
            "mined {} {}, picked up {} {}",
            self.broken,
            self.block,
            self.gained(cx),
            self.drop
        )
    }

    fn is_target(&self, cx: &GoalCx, pos: BlockPos) -> bool {
        cx.world
            .block_at(pos)
            .is_some_and(|n| kind_matches(&self.block, &n))
    }

    fn skip(&mut self, cx: &mut GoalCx, pos: BlockPos, why: &str) -> Step {
        self.skipped.push(pos);
        self.phase = Phase::Find;
        if self.skipped.len() >= MAX_SKIPPED {
            return Step::Failed(format!(
                "gave up on {} {} blocks ({why}); {}",
                self.skipped.len(),
                self.block,
                self.tally(cx)
            ));
        }
        cx.blocked(format!("skipping a {} block: {why}", self.block));
        Step::Bid(Plan::hold("goal_mine"))
    }

    /// Stand still, look at `pos` and break it, with the right tool in hand.
    fn dig_cmd(cx: &mut GoalCx, pos: BlockPos) -> Result<Plan, String> {
        let name = cx.world.block_at(pos).unwrap_or_default();
        match cx.world.tool_for(&name, cx.snap()) {
            ToolNeed::Missing(t) => return Err(format!("missing tool for {name}: needs {t}")),
            ToolNeed::Have(slot) => {
                if let Some(c) = equip(&cx.snap().me, slot) {
                    return Ok(Plan::new("goal_equip", c));
                }
            }
            ToolNeed::Hand => {}
        }
        let centre = pos.feet_centre() + Vec3::new(0.0, 0.5, 0.0);
        let c = Commands::default()
            .legs(LegsCmd::stop())
            .look_at(centre)
            .main(MainCmd::Dig(pos));
        Ok(Plan::new("goal_mine", c))
    }

    /// Counts the block being dug once it is gone.
    fn settle(&mut self, cx: &mut GoalCx) {
        let Phase::Dig { pos, .. } = self.phase else {
            return;
        };
        if self.is_target(cx, pos) {
            return;
        }
        self.broken += 1;
        let (g, n) = (self.gained(cx), self.count);
        cx.progress(g, n, format!("{}, {g} of {n} wanted", self.tally(cx)));
        self.phase = Phase::Advance { pos, ticks: 0 };
    }

    fn dig(&mut self, cx: &mut GoalCx, pos: BlockPos, ticks: u32) -> Step {
        if !self.is_target(cx, pos) {
            self.settle(cx);
            return Step::Bid(Plan::hold("goal_mine"));
        }
        if ticks > DIG_LIMIT_TICKS {
            return self.skip(cx, pos, "it did not break");
        }
        if !cx.preempted {
            self.phase = Phase::Dig {
                pos,
                ticks: ticks + 1,
            };
        }
        match Self::dig_cmd(cx, pos) {
            Ok(p) => Step::Bid(p),
            Err(e) => Step::Failed(e),
        }
    }

    /// After breaking a block at feet height: clear the one above it too and step in, so
    /// the body walks over the drop and the tunnel stays walkable.
    fn advance(&mut self, cx: &mut GoalCx, pos: BlockPos, ticks: u32) -> Step {
        let feet = cx.me_pos().block();
        let floor = cx.world.block_at(pos.below()).unwrap_or_default();
        let solid_floor = !floor.is_empty() && !replaceable(&floor);
        if pos.y != feet.y || !solid_floor || ticks > ADVANCE_TICKS + DIG_LIMIT_TICKS {
            self.pickup.arm();
            self.phase = Phase::Pickup(pos);
            return Step::Bid(Plan::hold("goal_mine"));
        }
        if !cx.preempted {
            self.phase = Phase::Advance {
                pos,
                ticks: ticks + 1,
            };
        }
        let head = pos.above();
        let head_name = cx.world.block_at(head).unwrap_or_default();
        if !replaceable(&head_name) && !head_name.is_empty() {
            if !natural(&head_name) || !target::safe(cx, head) {
                self.pickup.arm();
                self.phase = Phase::Pickup(pos);
                return Step::Bid(Plan::hold("goal_mine"));
            }
            return match Self::dig_cmd(cx, head) {
                Ok(p) => Step::Bid(p),
                Err(_) => {
                    self.phase = Phase::Pickup(pos);
                    Step::Bid(Plan::hold("goal_mine"))
                }
            };
        }
        let me = cx.me_pos();
        let c = pos.feet_centre();
        if (me - c).horizontal().len() < 0.3 || ticks > ADVANCE_TICKS + 20 {
            self.pickup.arm();
            self.phase = Phase::Pickup(pos);
            return Step::Bid(Plan::hold("goal_mine"));
        }
        let legs = LegsCmd::toward(heading_to(me, c), Speed::Walk);
        Step::Bid(Plan::new("goal_mine", Commands::default().legs(legs)))
    }
}

impl GoalTask for Mine {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        if self.held_at_start.is_none() {
            self.held_at_start = Some(cx.world.count_item(&self.drop));
        }
        self.settle(cx);
        let gained = self.gained(cx);
        if gained >= self.count {
            return Step::Done(self.tally(cx));
        }
        if self.broken >= self.count + LOSS_SLACK {
            return Step::Failed(format!("drops were lost: {}", self.tally(cx)));
        }
        if cx.world.free_slots() == 0 {
            return Step::Failed(format!("inventory full; {}", self.tally(cx)));
        }
        if self.phase == Phase::Find {
            if let ToolNeed::Missing(t) = cx.world.tool_for(&self.block, cx.snap()) {
                return Step::Failed(format!("missing tool for {}: needs {t}", self.block));
            }
            match target::choose(cx, &self.block, &mut self.skipped) {
                Some(p) => self.phase = Phase::Approach(p),
                None => {
                    return Step::Failed(format!(
                        "no {} within {SEARCH_RADIUS} blocks; {}",
                        self.block,
                        self.tally(cx)
                    ))
                }
            }
        }
        match self.phase {
            Phase::Find => Step::Bid(Plan::hold("goal_mine")),
            Phase::Approach(pos) => {
                if !self.is_target(cx, pos) {
                    self.phase = Phase::Find;
                    return Step::Bid(Plan::hold("goal_mine"));
                }
                if target::beside(cx.me_pos().block(), pos) {
                    self.walker.stop(cx);
                    self.phase = Phase::Dig { pos, ticks: 0 };
                    return self.dig(cx, pos, 0);
                }
                match self.walker.walk(cx, NavTarget::Beside { block: pos }, "goal_mine_walk") {
                    Walk::Arrived => {
                        self.phase = Phase::Dig { pos, ticks: 0 };
                        self.dig(cx, pos, 0)
                    }
                    Walk::Moving(plan) => Step::Bid(plan),
                    Walk::Failed(f) => self.skip(cx, pos, &format!("unreachable ({f:?})")),
                }
            }
            Phase::Dig { pos, ticks } => self.dig(cx, pos, ticks),
            Phase::Advance { pos, ticks } => self.advance(cx, pos, ticks),
            Phase::Pickup(pos) => match self.pickup.step(cx, pos) {
                Some(plan) => Step::Bid(plan),
                None => {
                    self.phase = Phase::Find;
                    Step::Bid(Plan::hold("goal_mine"))
                }
            },
        }
    }

    fn stop(&mut self, _nav: &mut dyn Navigator) {
        self.walker.reset();
        self.pickup.reset_walk();
        if let Phase::Approach(_) = self.phase {
            self.phase = Phase::Find;
        }
    }
}
