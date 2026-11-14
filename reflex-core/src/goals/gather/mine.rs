use crate::behaviours::common::heading_to;
use crate::channels::{Commands, LegsCmd, MainCmd, Speed};
use crate::goals::items::equip;
use crate::goals::ports::{kind_matches, NavTarget, Navigator, ToolNeed};
use crate::goals::task::{GoalCx, GoalTask, Plan, Step};
use crate::goals::travel::{Walk, Walker};
use crate::math::{BlockPos, Vec3};

pub const SEARCH_RADIUS: i32 = 32;
/// Stand within this distance of the block's centre to dig it (survival reach is 4.5).
const REACH: f64 = 4.0;
/// Digging one block for longer than this means something (a veto, a wrong tool) stops it.
pub const DIG_LIMIT_TICKS: u32 = 600;
const PICKUP_TICKS: u32 = 40;
/// Blocks given up on (unreachable or unbreakable) before the goal fails.
const MAX_SKIPPED: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Find,
    Approach(BlockPos),
    Dig { pos: BlockPos, ticks: u32 },
    Pickup { pos: BlockPos, ticks: u32 },
}

pub struct Mine {
    block: String,
    count: u32,
    mined: u32,
    phase: Phase,
    walker: Walker,
    skipped: Vec<BlockPos>,
}

impl Mine {
    pub fn new(block: String, count: u32) -> Self {
        Self {
            block,
            count: count.max(1),
            mined: 0,
            phase: Phase::Find,
            walker: Walker::default(),
            skipped: Vec::new(),
        }
    }

    fn is_target(&self, cx: &GoalCx, pos: BlockPos) -> bool {
        cx.world
            .block_at(pos)
            .is_some_and(|n| kind_matches(&self.block, &n))
    }

    fn skip(&mut self, cx: &mut GoalCx, pos: BlockPos, why: &str) -> Option<Step> {
        self.skipped.push(pos);
        self.phase = Phase::Find;
        if self.skipped.len() >= MAX_SKIPPED {
            return Some(Step::Failed(format!(
                "gave up on {} {} blocks ({why}); mined {} of {}",
                self.skipped.len(),
                self.block,
                self.mined,
                self.count
            )));
        }
        cx.blocked(format!("skipping a {} block: {why}", self.block));
        None
    }

    fn find(&mut self, cx: &mut GoalCx) -> Result<BlockPos, String> {
        if let ToolNeed::Missing(t) = cx.world.tool_for(&self.block, cx.snap()) {
            return Err(format!("missing tool for {}: needs {t}", self.block));
        }
        let from = cx.me_pos().block();
        cx.world
            .nearest_block(&self.block, from, SEARCH_RADIUS, &self.skipped)
            .ok_or_else(|| {
                format!(
                    "no {} within {SEARCH_RADIUS} blocks; mined {} of {}",
                    self.block, self.mined, self.count
                )
            })
    }

    fn dig(&mut self, cx: &mut GoalCx, pos: BlockPos, ticks: u32) -> Step {
        if !cx.preempted {
            self.phase = Phase::Dig {
                pos,
                ticks: ticks + 1,
            };
        }
        if ticks > DIG_LIMIT_TICKS {
            return self
                .skip(cx, pos, "it did not break")
                .unwrap_or(Step::Bid(Plan::hold("goal_mine")));
        }
        let tool = match cx.world.tool_for(&self.block, cx.snap()) {
            ToolNeed::Missing(t) => {
                return Step::Failed(format!("missing tool for {}: needs {t}", self.block))
            }
            ToolNeed::Have(slot) => equip(&cx.snap().me, slot),
            ToolNeed::Hand => None,
        };
        if let Some(c) = tool {
            return Step::Bid(Plan::new("goal_equip", c));
        }
        let centre = pos.feet_centre() + Vec3::new(0.0, 0.5, 0.0);
        let c = Commands::default()
            .legs(LegsCmd::stop())
            .look_at(centre)
            .main(MainCmd::Dig(pos));
        Step::Bid(Plan::new("goal_mine", c))
    }
}

impl GoalTask for Mine {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        if self.mined >= self.count {
            return Step::Done(format!("mined {} {}", self.mined, self.block));
        }
        if cx.world.free_slots() == 0 {
            return Step::Failed(format!(
                "inventory full; mined {} of {} {}",
                self.mined, self.count, self.block
            ));
        }
        if self.phase == Phase::Find {
            match self.find(cx) {
                Ok(p) => self.phase = Phase::Approach(p),
                Err(e) => return Step::Failed(e),
            }
        }
        match self.phase {
            Phase::Find => Step::Bid(Plan::hold("goal_mine")),
            Phase::Approach(pos) => {
                if !self.is_target(cx, pos) {
                    self.phase = Phase::Find;
                    return Step::Bid(Plan::hold("goal_mine"));
                }
                let t = NavTarget::Block { pos, radius: REACH };
                match self.walker.walk(cx, t, "goal_mine_walk") {
                    Walk::Arrived => {
                        self.phase = Phase::Dig { pos, ticks: 0 };
                        self.dig(cx, pos, 0)
                    }
                    Walk::Moving(plan) => Step::Bid(plan),
                    Walk::Failed(f) => self
                        .skip(cx, pos, &format!("unreachable ({f:?})"))
                        .unwrap_or(Step::Bid(Plan::hold("goal_mine"))),
                }
            }
            Phase::Dig { pos, ticks } => {
                if self.is_target(cx, pos) {
                    return self.dig(cx, pos, ticks);
                }
                self.mined += 1;
                let (m, n) = (self.mined, self.count);
                cx.progress(m, n, format!("mined {m}/{n} {}", self.block));
                self.phase = Phase::Pickup { pos, ticks: 0 };
                Step::Bid(Plan::hold("goal_mine"))
            }
            Phase::Pickup { pos, ticks } => {
                let me = cx.me_pos();
                let drop = pos.feet_centre();
                if (drop - me).horizontal().len() < 0.5 || ticks > PICKUP_TICKS {
                    self.phase = Phase::Find;
                    return Step::Bid(Plan::hold("goal_mine"));
                }
                if !cx.preempted {
                    self.phase = Phase::Pickup {
                        pos,
                        ticks: ticks + 1,
                    };
                }
                let legs = LegsCmd::toward(heading_to(me, drop), Speed::Walk);
                Step::Bid(Plan::new("goal_pickup", Commands::default().legs(legs)))
            }
        }
    }

    fn stop(&mut self, _nav: &mut dyn Navigator) {
        self.walker.reset();
        if let Phase::Approach(_) = self.phase {
            self.phase = Phase::Find;
        }
    }
}
