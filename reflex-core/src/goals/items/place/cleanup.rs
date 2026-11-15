//! Taking down the scaffold a placement stood on, highest block first, so the body never
//! breaks what it still needs and steps down one block at a time off its own pillar.

use crate::channels::{Commands, LegsCmd, MainCmd};
use crate::goals::items::{equip, replaceable};
use crate::goals::ports::{NavTarget, ToolNeed};
use crate::goals::task::{GoalCx, Plan};
use crate::goals::travel::{Walk, Walker};
use crate::math::{BlockPos, Vec3};

const REACH: f64 = 4.0;
/// Ticks one scaffold block may take before it is left standing.
const BLOCK_TICKS: u32 = 300;

#[derive(Default)]
pub struct Cleanup {
    left: Vec<BlockPos>,
    walker: Walker,
    ticks: u32,
    /// Blocks taken down and blocks left standing.
    pub removed: u32,
    pub left_standing: u32,
}

impl Cleanup {
    pub fn add(&mut self, blocks: impl IntoIterator<Item = BlockPos>) {
        for b in blocks {
            if !self.left.contains(&b) {
                self.left.push(b);
            }
        }
        self.left.sort_by_key(|b| b.y);
    }

    /// Scaffold blocks still to take down.
    pub fn pending(&self) -> &[BlockPos] {
        &self.left
    }

    pub fn reset_walk(&mut self) {
        self.walker.reset();
    }

    /// Counts the listed blocks that are gone already (dug ahead of the cleanup by `dig`).
    pub fn tally(&mut self, cx: &GoalCx) {
        let before = self.left.len();
        self.left
            .retain(|b| cx.world.block_at(*b).is_some_and(|k| !replaceable(&k)));
        self.removed += (before - self.left.len()) as u32;
    }

    /// One tick of cleanup; `None` once every scaffold block is gone or given up.
    pub fn step(&mut self, cx: &mut GoalCx) -> Option<Plan> {
        loop {
            let &b = self.left.last()?;
            let standing = cx.world.block_at(b).is_some_and(|k| !replaceable(&k));
            if !standing {
                self.left.pop();
                self.removed += 1;
                self.ticks = 0;
                continue;
            }
            if self.ticks > BLOCK_TICKS {
                self.left.pop();
                self.left_standing += 1;
                self.ticks = 0;
                continue;
            }
            break;
        }
        let b = *self.left.last()?;
        if !cx.preempted {
            self.ticks += 1;
        }
        let me = cx.snap().me.clone();
        let centre = b.feet_centre() + Vec3::new(0.0, 0.5, 0.0);
        if me.eye().dist(centre) > REACH {
            let t = NavTarget::Block {
                pos: b,
                radius: REACH - 1.0,
            };
            return match self.walker.walk(cx, t, "goal_place_cleanup") {
                Walk::Moving(p) => Some(p),
                Walk::Arrived => Some(Plan::hold("goal_place_cleanup")),
                Walk::Failed(_) => {
                    self.ticks = BLOCK_TICKS + 1;
                    Some(Plan::hold("goal_place_cleanup"))
                }
            };
        }
        self.walker.stop(cx);
        Some(self.dig(cx, b))
    }

    /// The plan that digs scaffold block `b` from where the body stands.
    pub fn dig(&mut self, cx: &mut GoalCx, b: BlockPos) -> Plan {
        let me = cx.snap().me.clone();
        let centre = b.feet_centre() + Vec3::new(0.0, 0.5, 0.0);
        // Taking its own scaffold down is explicit: the ledger lets go of it first.
        cx.ledger.forget(b);
        let name = cx.world.block_at(b).unwrap_or_default();
        if let ToolNeed::Have(slot) = cx.world.tool_for(&name, cx.snap()) {
            if let Some(c) = equip(&me, slot) {
                return Plan::new("goal_equip", c);
            }
        }
        let c = Commands::default()
            .legs(LegsCmd::stop())
            .look_at(centre)
            .main(MainCmd::Dig(b));
        Plan::new("goal_place_cleanup", c)
    }
}
