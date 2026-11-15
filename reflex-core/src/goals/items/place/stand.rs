//! Where to stand to place a block: a spot whose eye sees a face of a support of the cell
//! and is within reach, on the floor if one works, else on a short pillar the route
//! planner builds (one or more scaffold blocks under the feet). Live, every roof block of
//! the hut was refused with "stand higher" and nothing acted on it.

use super::super::surfaces::{face_visible, opens_on_click, replaceable};
use super::footprint::{extra_cell, Extra};
use crate::goals::task::GoalCx;
use crate::math::{BlockPos, Vec3};

/// Eye to the clicked face's middle, at most (survival reach is 4.5).
pub const REACH: f64 = 4.4;
/// Scaffold blocks one spot may need under it.
const MAX_PILLAR: u32 = 3;
/// Extra cost per scaffold block, in blocks of walking.
const PILLAR_COST: f64 = 4.0;
/// How much a planned bed facing must lean to one side (about 34 degrees off the axis at
/// most), so the body landing off the spot's centre still faces the same side.
const PLAN_MARGIN: f64 = 1.5;
/// At the click the actual eye decides; only an exact diagonal is refused.
pub const CLICK_MARGIN: f64 = 1.0;

const FACES: [(i32, i32, i32); 6] = [
    (0, -1, 0),
    (1, 0, 0),
    (-1, 0, 0),
    (0, 0, 1),
    (0, 0, -1),
    (0, 1, 0),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub feet: BlockPos,
    /// Scaffold blocks needed under the feet.
    pub pillar: u32,
}

/// Whether a player standing at `pos` has any part of its hitbox (0.6 wide, 1.8 tall) in
/// `cell`, with a small margin. The server refuses a block whose shape would hold a player,
/// and the walk to a spot ends up to half a block off its centre: live, a bed went unplaced
/// eight clicks in a row with the body 0.05 into the foot cell.
pub fn body_overlaps(pos: Vec3, cell: BlockPos) -> bool {
    const HALF: f64 = 0.3 + 0.02;
    const HEIGHT: f64 = 1.8;
    let (x, y, z) = (f64::from(cell.x), f64::from(cell.y), f64::from(cell.z));
    pos.x + HALF > x
        && pos.x - HALF < x + 1.0
        && pos.z + HALF > z
        && pos.z - HALF < z + 1.0
        && pos.y + HEIGHT > y
        && pos.y < y + 1.0
}

pub fn eye_at(feet: BlockPos) -> Vec3 {
    feet.feet_centre() + Vec3::new(0.0, 1.62, 0.0)
}

pub fn face_point(cell: BlockPos, support: BlockPos) -> Vec3 {
    let c = |b: BlockPos| b.feet_centre() + Vec3::new(0.0, 0.5, 0.0);
    (c(cell) + c(support)) * 0.5
}

/// Non-replaceable neighbours of `cell` that take a block on a click.
pub fn supports(cx: &GoalCx, cell: BlockPos) -> Vec<BlockPos> {
    FACES
        .iter()
        .map(|&(x, y, z)| cell.offset(x, y, z))
        .filter(|&n| {
            cx.world
                .block_at(n)
                .is_some_and(|k| !replaceable(&k) && !opens_on_click(&k))
        })
        .collect()
}

/// A support whose face an eye at `eye` sees, within reach, with room for all of `block`
/// when it takes two cells (`margin`: see `footprint::facing`).
pub fn visible_support(
    cx: &GoalCx,
    block: &str,
    eye: Vec3,
    cell: BlockPos,
    supports: &[BlockPos],
    margin: f64,
) -> Option<BlockPos> {
    supports
        .iter()
        .copied()
        .filter(|&s| face_visible(eye, s, cell) && eye.dist(face_point(cell, s)) <= REACH)
        .filter(|&s| match extra_cell(block, eye, cell, s, margin) {
            Extra::Nothing => true,
            Extra::Cell(p) => free(cx, p),
            Extra::Unclear => false,
        })
        .min_by(|&a, &b| {
            let d = |s| eye.dist(face_point(cell, s));
            d(a).total_cmp(&d(b))
        })
}

/// A neighbour that would open a menu when clicked, for saying why nothing else will do.
pub fn opener_beside(cx: &GoalCx, cell: BlockPos) -> Option<String> {
    FACES
        .iter()
        .filter_map(|&(x, y, z)| cx.world.block_at(cell.offset(x, y, z)))
        .find(|k| opens_on_click(k))
}

fn free(cx: &GoalCx, p: BlockPos) -> bool {
    cx.world
        .block_at(p)
        .is_some_and(|k| replaceable(&k) && k != "water" && k != "lava")
}

/// The best spot to place into `cell`, never with the body or a pillar in `cell` or in any
/// of `keep_clear` (cells still to fill).
pub fn find(
    cx: &GoalCx,
    block: &str,
    cell: BlockPos,
    keep_clear: &[BlockPos],
    scaffold: u32,
) -> Option<Spot> {
    let sup = supports(cx, cell);
    if sup.is_empty() {
        return None;
    }
    let me = cx.me_pos();
    let here = me.block();
    let mut best: Option<(f64, Spot)> = None;
    for dy in -3..=2 {
        for dx in -3..=3 {
            for dz in -3..=3 {
                let feet = cell.offset(dx, dy, dz);
                let body = [feet, feet.above()];
                if body.contains(&cell) || body.iter().any(|b| keep_clear.contains(b)) {
                    continue;
                }
                if visible_support(cx, block, eye_at(feet), cell, &sup, PLAN_MARGIN).is_none() {
                    continue;
                }
                if !body.iter().all(|&b| free(cx, b)) {
                    continue;
                }
                let mut pillar = 0;
                let mut below = feet.below();
                while free(cx, below) && pillar <= MAX_PILLAR {
                    if below == cell || keep_clear.contains(&below) {
                        pillar = MAX_PILLAR + 1;
                        break;
                    }
                    pillar += 1;
                    below = below.below();
                }
                let floor = cx.world.block_at(below).unwrap_or_default();
                if pillar > MAX_PILLAR.min(scaffold) || floor.is_empty() || replaceable(&floor) {
                    continue;
                }
                let walk = if feet == here {
                    0.0
                } else {
                    me.dist(feet.feet_centre()) + 1.0
                };
                let cost = walk + PILLAR_COST * f64::from(pillar);
                if best.is_none_or(|(c, _)| cost < c) {
                    best = Some((cost, Spot { feet, pillar }));
                }
            }
        }
    }
    best.map(|(_, s)| s)
}
