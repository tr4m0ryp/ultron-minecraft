//! Moves along the ground: walk or dig into the next column, step or drop down, dig a
//! staircase down, bridge a gap, and walk diagonally across open ground.

use super::{Edge, Moves, DIAGONAL, FALL_PER_BLOCK, JUMP, PLACE, PLACE_PENALTY, SWIM, WALK};
use crate::goals::travel::route::cell::Cell;
use crate::goals::travel::route::hop::{Hop, HopKind, Placement};
use crate::math::BlockPos;

impl Moves<'_> {
    /// Into column `c` at the same level, then whatever its floor allows: stand, dig a step
    /// down (never under the body), drop, or bridge the gap.
    fn forward(
        &self,
        n: BlockPos,
        c: BlockPos,
        solid_floor: bool,
        scaffold: u32,
        out: &mut Vec<Edge>,
    ) {
        let mut dig = Vec::new();
        let Some(body) = self.clear_body(c, &mut dig) else {
            return;
        };
        if matches!(self.t.cell(c), Cell::Hazard | Cell::Lava) {
            return;
        }
        let below = self.t.cell(c.below());
        if below.is_floor() {
            let hop = Hop::walk(n, c);
            Self::push_dug(out, hop, dig.clone(), WALK + body);
        }
        // Staircase down: break the floor ahead, stand one lower on what is under it.
        if below.is_solid() && self.t.cell(c.offset(0, -2, 0)).is_floor() {
            let mut d2 = dig.clone();
            if let Some(k) = self.clear(c.below(), false, &mut d2) {
                let to = c.below();
                Self::push_dug(out, Hop::walk(n, to), d2, WALK + body + k + JUMP);
            }
        }
        if !dig.is_empty() || below.is_solid() {
            return;
        }
        if below.takes_block() {
            self.drop_down(n, c, out);
            if scaffold > 0 && solid_floor {
                let hop = Hop {
                    from: n,
                    to: c,
                    kind: HopKind::Bridge,
                    dig: Vec::new(),
                    place: Some(Placement {
                        cell: c.below(),
                        against: n.below(),
                    }),
                };
                out.push(Edge {
                    hop,
                    cost: WALK + PLACE + PLACE_PENALTY + JUMP,
                });
            }
        }
    }

    fn drop_down(&self, n: BlockPos, c: BlockPos, out: &mut Vec<Edge>) {
        let mut feet = c;
        for k in 1..=self.o.max_drop.max(1) {
            feet = feet.below();
            match self.t.cell(feet) {
                Cell::Open => {}
                Cell::Water => {
                    let cost = WALK + FALL_PER_BLOCK * k as f32 + SWIM;
                    Self::push(out, n, feet, HopKind::Walk, cost);
                    return;
                }
                _ => return,
            }
            if self.t.cell(feet.below()).is_floor() {
                let cost = WALK + JUMP + FALL_PER_BLOCK * k as f32;
                Self::push(out, n, feet, HopKind::Walk, cost);
                return;
            }
        }
    }

    fn diagonal(&self, n: BlockPos, dx: i32, dz: i32, out: &mut Vec<Edge>) {
        let t = n.offset(dx, 0, dz);
        let open = |p: BlockPos| self.t.cell(p).is_open() && self.t.cell(p.above()).is_open();
        if open(t)
            && open(n.offset(dx, 0, 0))
            && open(n.offset(0, 0, dz))
            && self.t.cell(t.below()).is_floor()
        {
            Self::push(out, n, t, HopKind::Walk, DIAGONAL);
        }
    }
}
