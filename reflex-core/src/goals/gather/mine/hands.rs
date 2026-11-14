//! The mining hand work: breaking a block with the right tool, and stepping into the gap
//! it left (clearing the block above too) so the body walks over the drop.

use super::{Mine, Phase, ADVANCE_TICKS, DIG_LIMIT_TICKS};
use super::target;
use crate::behaviours::common::heading_to;
use crate::channels::{Commands, LegsCmd, MainCmd, Speed};
use crate::goals::items::{equip, replaceable};
use crate::goals::ports::ToolNeed;
use crate::goals::task::{GoalCx, Plan, Step};
use crate::goals::travel::route::natural;
use crate::math::{BlockPos, Vec3};

impl Mine {
    /// Stand still, look at `pos` and break it, with the right tool in hand.
    pub(super) fn dig_cmd(cx: &mut GoalCx, pos: BlockPos) -> Result<Plan, String> {
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

    /// After breaking a block at feet height: clear the one above it too and step in, so
    /// the body walks over the drop and the tunnel stays walkable.
    pub(super) fn advance(&mut self, cx: &mut GoalCx, pos: BlockPos, ticks: u32) -> Step {
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
