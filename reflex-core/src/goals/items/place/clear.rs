//! Clearing the cell a placement targets when something is already in it: leaves, dirt,
//! grass, sand, snow, flowers and other soft natural blocks are dug out and the block goes
//! in. Live, four places failed "is occupied by leaves/dirt" and the thought layer had to
//! mine the cell by hand first. A block the player placed, a protected region and the
//! surroundings of a block the companion placed are left alone unless the goal says
//! `replace`.

use super::{Place, Step};
use crate::goals::items::dig_out::dig_out;
use crate::goals::items::surfaces::{replaceable, soft_natural};
use crate::goals::task::GoalCx;
use crate::math::BlockPos;

/// What a placement may do about the block in its cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Clearance {
    /// The cell takes the block as it is (air, water, plants that give way).
    Open,
    /// Dig what is there, then place.
    Clear,
    /// Leave it; the reason names what holds the cell.
    Refuse(String),
}

/// Decides what a placement at `pos` does about `held`, the block in the cell. `replace` is
/// the goal's explicit opt-in to clear anything breakable, placed or protected.
pub fn clearance(cx: &GoalCx, pos: BlockPos, held: &str, replace: bool) -> Clearance {
    if replaceable(held) {
        return Clearance::Open;
    }
    if replace {
        return Clearance::Clear;
    }
    let at = format!("{} {} {}", pos.x, pos.y, pos.z);
    if cx.ledger.is_placed(pos) {
        return Clearance::Refuse(format!(
            "{at} holds {held} that a player placed; add replace to clear it"
        ));
    }
    if cx.knobs().protected.iter().any(|r| r.contains(pos)) {
        return Clearance::Refuse(format!(
            "{at} holds {held} inside a protected region; add replace to clear it"
        ));
    }
    if cx.ledger.blocks_dig(pos) {
        return Clearance::Refuse(format!(
            "{at} holds {held} beside blocks a player placed; add replace to clear it"
        ));
    }
    if soft_natural(held) {
        Clearance::Clear
    } else {
        Clearance::Refuse(format!("{at} is occupied by {held}"))
    }
}

impl Place {
    /// One tick of clearing the target cell: walk within reach, then dig it.
    pub(super) fn clear(&mut self, cx: &mut GoalCx, held: &str) -> Step {
        let pos = self.pos;
        match dig_out(
            cx,
            &mut self.walker,
            &mut self.clear_ticks,
            pos,
            held,
            "goal_place_clear",
        ) {
            Ok(plan) => Step::Bid(plan),
            Err(why) => self.conclude(cx, Step::Failed(why)),
        }
    }
}
