//! The moves a player has, with their costs in ticks. Walking is cheapest; every dig and
//! every placed block carries a penalty on top of its time, so a walkable route wins
//! unless it is much longer (about 15 blocks of detour per dug block, 4 per placed one).

use super::cell::{Cell, Region, Terrain};
use super::hop::{Hop, HopKind, Placement};
use crate::math::BlockPos;

pub const WALK: f32 = 4.63;
const DIAGONAL: f32 = WALK * std::f32::consts::SQRT_2;
const JUMP: f32 = 2.0;
const FALL_PER_BLOCK: f32 = 1.0;
const SWIM: f32 = 8.0;
/// Added to each dug block's ticks: tool swaps, the drop, the risk of what lies behind.
const DIG_PENALTY: f32 = 30.0;
/// Extra when breaking a block below sand or gravel, which then has to be dug again.
const FALL_REDIG: f32 = 60.0;
/// One placed block: aiming and the server round trip, plus the block it spends.
const PLACE: f32 = 6.0;
const PLACE_PENALTY: f32 = 12.0;

const CARDINAL: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const DIAGONALS: [(i32, i32); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

#[derive(Clone, Debug, PartialEq)]
pub struct RouteOpts {
    /// Deepest drop a move may take (the edge guard's safe drop).
    pub max_drop: i32,
    /// Scaffold blocks at hand; pillars and bridges spend them.
    pub scaffold: u32,
    /// The route may break natural blocks in its way.
    pub dig: bool,
    /// Never break anything inside these.
    pub protected: Vec<Region>,
    /// Nodes expanded before the search returns its best partial route.
    pub max_nodes: usize,
    /// Heuristic weight: above 1 the search heads for the goal greedily, which keeps it
    /// from flooding solid ground when every route needs digging.
    pub weight: f32,
}

impl Default for RouteOpts {
    fn default() -> Self {
        Self {
            max_drop: 2,
            scaffold: 0,
            dig: true,
            protected: Vec::new(),
            max_nodes: 30_000,
            weight: 2.5,
        }
    }
}

impl RouteOpts {
    /// Walking only: the old planner's moves, used to tell "enclosed" from "far".
    pub fn walk_only(max_drop: i32) -> Self {
        Self {
            max_drop,
            dig: false,
            scaffold: 0,
            weight: 1.0,
            ..Self::default()
        }
    }
}

pub struct Edge {
    pub hop: Hop,
    pub cost: f32,
}

pub struct Moves<'a> {
    pub t: &'a dyn Terrain,
    pub o: &'a RouteOpts,
}

impl Moves<'_> {
    fn may_dig(&self, p: BlockPos) -> bool {
        self.o.dig && !self.o.protected.iter().any(|r| r.contains(p))
    }

    /// Breaking `p` must not open water or lava onto the body: no liquid above or beside.
    fn dig_safe(&self, p: BlockPos) -> bool {
        [
            p.above(),
            p.offset(1, 0, 0),
            p.offset(-1, 0, 0),
            p.offset(0, 0, 1),
            p.offset(0, 0, -1),
        ]
        .iter()
        .all(|&q| !matches!(self.t.cell(q), Cell::Water | Cell::Lava | Cell::Unknown))
    }

    /// Ticks to make `p` passable for the body, with the block to dig if any. `overhead`:
    /// the cell is straight above the body, where a falling block would land on its head.
    fn clear(&self, p: BlockPos, overhead: bool, dig: &mut Vec<BlockPos>) -> Option<f32> {
        match self.t.cell(p) {
            Cell::Open => Some(0.0),
            Cell::Water => Some(SWIM),
            Cell::Solid {
                dig: Some(ticks), ..
            } if self.may_dig(p) && self.dig_safe(p) => {
                let falls_above = matches!(self.t.cell(p.above()), Cell::Solid { falls: true, .. });
                if falls_above && overhead {
                    return None;
                }
                dig.push(p);
                let redig = if falls_above { FALL_REDIG } else { 0.0 };
                Some(f32::from(ticks) + DIG_PENALTY + redig)
            }
            _ => None,
        }
    }

    /// Body cells at `feet` and above it made passable.
    fn clear_body(&self, feet: BlockPos, dig: &mut Vec<BlockPos>) -> Option<f32> {
        Some(self.clear(feet.above(), false, dig)? + self.clear(feet, false, dig)?)
    }

    /// The body could stand at `n`: floor below (or placed by the route) and no harm there.
    fn has_floor(&self, n: BlockPos, placed: bool) -> bool {
        placed || self.t.cell(n.below()).is_floor()
    }

    fn push(out: &mut Vec<Edge>, from: BlockPos, to: BlockPos, kind: HopKind, cost: f32) {
        out.push(Edge {
            hop: Hop {
                from,
                to,
                kind,
                dig: Vec::new(),
                place: None,
            },
            cost,
        });
    }

    fn push_dug(out: &mut Vec<Edge>, mut hop: Hop, mut dig: Vec<BlockPos>, cost: f32) {
        dig.sort_by_key(|p| std::cmp::Reverse(p.y));
        dig.dedup();
        hop.dig = dig;
        out.push(Edge { hop, cost });
    }

    /// Every move out of standing cell `n`. `placed`: n's floor is a block this route put
    /// there; `scaffold`: blocks still in hand.
    pub fn from(&self, n: BlockPos, placed: bool, scaffold: u32, out: &mut Vec<Edge>) {
        let swimming = self.t.cell(n) == Cell::Water;
        let solid_floor = self.has_floor(n, placed);
        for (dx, dz) in CARDINAL {
            let c = n.offset(dx, 0, dz);
            self.forward(n, c, solid_floor, scaffold, out);
            if !swimming {
                self.ascend(n, c, out);
            }
        }
        for (dx, dz) in DIAGONALS {
            self.diagonal(n, dx, dz, out);
        }
        if scaffold > 0 && solid_floor && !swimming {
            self.pillar(n, out);
        }
    }

    /// Into column `c` at the same level, then whatever its floor allows: stand, dig a step
    /// down (never under the body), drop, or bridge the gap.
    fn forward(&self, n: BlockPos, c: BlockPos, solid_floor: bool, scaffold: u32, out: &mut Vec<Edge>) {
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
            Self::push_dug(out, hop, dig, WALK + body);
            return;
        }
        // Staircase down: break the floor ahead, stand one lower on what is under it.
        if below.is_solid() && self.t.cell(c.offset(0, -2, 0)).is_floor() {
            let mut d2 = dig.clone();
            if let Some(k) = self.clear(c.below(), false, &mut d2) {
                let to = c.below();
                Self::push_dug(out, Hop::walk(n, to), d2, WALK + body + k + JUMP);
            }
        }
        if !dig.is_empty() {
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
