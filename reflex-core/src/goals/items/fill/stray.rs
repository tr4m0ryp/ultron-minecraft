//! Scaffold left under the fill. A body that starts a roof inside its walls has to pillar
//! out, and the block it stood on then sits under the roof being laid: live, one dirt block
//! per hut stayed sealed inside, out of reach of the cleanup at the end. Such a block is
//! taken down as soon as the body is out from under the fill and can reach it from where it
//! stands (the wall top it stepped onto), before the roof closes over it.

use crate::goals::items::hand_sees;
use crate::goals::task::GoalCx;
use crate::math::{BlockPos, Vec3};

/// Eye to the top of the block, at most (survival reach is 4.5).
const REACH: f64 = 4.3;

fn under(cover: &[BlockPos], p: BlockPos) -> bool {
    cover.iter().any(|c| c.x == p.x && c.z == p.z && c.y > p.y)
}

/// A scaffold block among `scaffold` that lies under the fill's cells and that the body,
/// itself out from under them and not standing on it, can dig from here.
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
            && feet.below() != b
            && feet != b
            && eye.dist(top) <= REACH
            && hand_sees(&*cx.world, eye, top, &[b])
    })
}
