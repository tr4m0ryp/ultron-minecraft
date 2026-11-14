//! Moves upward: step up a block (digging head room and the step as needed, which makes a
//! staircase out of a pit) and pillar up on a placed block.

use super::{Edge, Moves, JUMP, PLACE, PLACE_PENALTY, WALK};
use crate::goals::travel::route::cell::Cell;
use crate::goals::travel::route::hop::{Hop, HopKind, Placement};
use crate::math::BlockPos;

impl Moves<'_> {
    /// One block up into column `c`, digging head room and the step as needed (a staircase
    /// out of a pit when the step is dug into the pit wall).
    fn ascend(&self, n: BlockPos, c: BlockPos, out: &mut Vec<Edge>) {
        if !self.t.cell(c).is_floor() {
            return;
        }
        let mut dig = Vec::new();
        let Some(head) = self.clear(n.offset(0, 2, 0), true, &mut dig) else {
            return;
        };
        let to = c.above();
        let Some(body) = self.clear_body(to, &mut dig) else {
            return;
        };
        if matches!(self.t.cell(to), Cell::Hazard | Cell::Lava) {
            return;
        }
        Self::push_dug(out, Hop::walk(n, to), dig, WALK + JUMP + head + body);
    }

    /// Jump and place a block under the feet.
    fn pillar(&self, n: BlockPos, out: &mut Vec<Edge>) {
        let mut dig = Vec::new();
        let Some(head) = self.clear(n.offset(0, 2, 0), true, &mut dig) else {
            return;
        };
        if self.t.cell(n.offset(0, 2, 0)) == Cell::Water {
            return;
        }
        let hop = Hop {
            from: n,
            to: n.above(),
            kind: HopKind::Pillar,
            dig: Vec::new(),
            place: Some(Placement {
                cell: n,
                against: n.below(),
            }),
        };
        Self::push_dug(out, hop, dig, JUMP + PLACE + PLACE_PENALTY + head);
    }
}
