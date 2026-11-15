//! The body stands in the cell it is about to fill. Usually a step aside clears it. Live,
//! closing a roof from inside, the body stood on its own scaffold block in a corner with
//! the roof it had just laid at head height on every open side: no step aside existed, and
//! it pushed against the wall for minutes. The scaffold under the feet then comes down
//! first, and the body drops out of the cell.

use crate::behaviours::common::away_from;
use crate::channels::{Commands, LegsCmd, MainCmd, Speed};
use crate::goals::task::{GoalCx, Plan};
use crate::math::{BlockPos, Vec3};

/// Ticks of stepping aside before the scaffold under the feet is taken down.
const ASIDE_TICKS: u32 = 20;

#[derive(Debug, Default)]
pub struct InWay {
    ticks: u32,
}

impl InWay {
    pub fn reset(&mut self) {
        self.ticks = 0;
    }

    /// One tick with the body in `cell`. `scaffold` tells whether a block is scaffold this
    /// goal put up (and may take down).
    pub fn step(
        &mut self,
        cx: &mut GoalCx,
        cell: BlockPos,
        scaffold: impl Fn(BlockPos) -> bool,
    ) -> Plan {
        cx.blocked("standing in the spot to fill");
        let me = cx.me_pos();
        self.ticks += u32::from(!cx.preempted);
        let under = (me - Vec3::new(0.0, 0.5, 0.0)).block();
        if self.ticks > ASIDE_TICKS && scaffold(under) {
            // Taking its own scaffold down is explicit: the ledger lets go of it first.
            cx.ledger.forget(under);
            let centre = under.feet_centre() + Vec3::new(0.0, 0.5, 0.0);
            let dig = Commands::default()
                .legs(LegsCmd::stop())
                .look_at(centre)
                .main(MainCmd::Dig(under));
            return Plan::new("goal_place", dig);
        }
        let legs = LegsCmd::toward(away_from(me, cell.feet_centre()), Speed::Walk);
        Plan::new("goal_place", Commands::default().legs(legs))
    }
}
