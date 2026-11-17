//! Scaffold left under the fill. A body that starts a roof inside its walls has to pillar
//! out, and the block it stood on then sits under the roof being laid: live, one dirt block
//! per hut stayed sealed inside, out of reach of the cleanup at the end. Such a block is
//! taken down as soon as the body is out from under the fill and can reach it from where it
//! stands (the wall top it stepped onto), before the roof closes over it.

use crate::goals::items::{hand_sees, replaceable};
use crate::goals::task::GoalCx;
use crate::math::{BlockPos, Vec3};

/// Eye to the top of the block, at most (survival reach is 4.5).
const REACH: f64 = 4.3;

fn under(cover: &[BlockPos], p: BlockPos) -> bool {
    cover.iter().any(|c| c.x == p.x && c.z == p.z && c.y > p.y)
}

/// A scaffold block among `scaffold` that lies under the fill's cells and that the body,
/// itself out from under them and not standing on it, can dig from here. Only a block
/// that stands: the list is a record of clicks and digs, and a cell on it that is air (the
/// server refused the block, or the dig is done) is nothing to dig.
pub fn reachable(cx: &GoalCx, cover: &[BlockPos], scaffold: &[BlockPos]) -> Option<BlockPos> {
    let me = &cx.snap().me;
    let feet = me.pos.block();
    if !me.on_ground || under(cover, feet) {
        return None;
    }
    let eye = me.eye();
    scaffold.iter().copied().find(|&b| {
        let top = b.feet_centre() + Vec3::new(0.0, 0.95, 0.0);
        under(cover, b)
            && cx.world.block_at(b).is_some_and(|k| !replaceable(&k))
            && feet.below() != b
            && feet != b
            && eye.dist(top) <= REACH
            && hand_sees(&*cx.world, eye, top, &[b])
    })
}

/// Ticks one such block may take. Live, a pillar cell stayed on the list after its block
/// was gone, and the dig at the empty cell never ended; `reachable` now asks for a
/// standing block, and this bound stays for a block that will not break.
const DIG_TICKS: u32 = 60;

/// Bounds the digging per block; a block that outlasts it is left to the cleanup at the end.
#[derive(Debug, Default)]
pub struct Budget {
    current: Option<(BlockPos, u32)>,
    gave_up: Vec<BlockPos>,
}

impl Budget {
    pub fn given_up(&self, b: BlockPos) -> bool {
        self.gave_up.contains(&b)
    }

    /// One more tick on `b`; false once its ticks are spent.
    pub fn spend(&mut self, b: BlockPos, preempted: bool) -> bool {
        let ticks = match &mut self.current {
            Some((cur, t)) if *cur == b => t,
            slot => &mut slot.insert((b, 0)).1,
        };
        *ticks += u32::from(!preempted);
        if *ticks > DIG_TICKS {
            self.gave_up.push(b);
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_block_that_will_not_break_is_given_up_after_its_ticks() {
        let (a, b) = (BlockPos::new(1, 64, 1), BlockPos::new(2, 64, 1));
        let mut budget = Budget::default();
        assert!((0..DIG_TICKS).all(|_| budget.spend(a, false)));
        assert!(
            budget.spend(a, true),
            "a tick a reflex held the body does not count"
        );
        assert!(!budget.spend(a, false) && budget.given_up(a));
        assert!(budget.spend(b, false) && !budget.given_up(b));
    }
}
