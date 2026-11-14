//! Which block to mine next: one level with the body (head height first, then feet) in a
//! neighbouring column, so mining advances a walkable tunnel; otherwise the nearest one,
//! reached from the side by the route planner. Never a block that would let water or lava
//! in, never one inside a protected region.

use crate::goals::ports::kind_matches;
use crate::goals::task::GoalCx;
use crate::math::BlockPos;

pub const SEARCH_RADIUS: i32 = 32;
const CARDINAL: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
/// Unsafe candidates skipped in one search before it gives up.
const MAX_REJECTS: usize = 8;

fn liquid(name: &str) -> bool {
    matches!(name, "water" | "lava") || name.ends_with("_water") || name.ends_with("_lava")
}

/// Breaking `p` keeps water and lava out and leaves protected builds alone.
pub fn safe(cx: &GoalCx, p: BlockPos) -> bool {
    if cx.knobs().protected.iter().any(|r| r.contains(p)) {
        return false;
    }
    [
        p.above(),
        p.offset(1, 0, 0),
        p.offset(-1, 0, 0),
        p.offset(0, 0, 1),
        p.offset(0, 0, -1),
    ]
    .iter()
    .all(|&q| cx.world.block_at(q).is_none_or(|n| !liquid(&n)))
}

fn is(cx: &GoalCx, kind: &str, p: BlockPos) -> bool {
    cx.world.block_at(p).is_some_and(|n| kind_matches(kind, &n))
}

/// The block level with the body and next to it, if one is the kind.
pub fn level_neighbour(cx: &GoalCx, kind: &str, skipped: &[BlockPos]) -> Option<BlockPos> {
    let feet = cx.me_pos().block();
    CARDINAL.iter().find_map(|&(dx, dz)| {
        [1, 0].iter().find_map(|&dy| {
            let p = feet.offset(dx, dy, dz);
            (is(cx, kind, p) && !skipped.contains(&p) && safe(cx, p)).then_some(p)
        })
    })
}

pub fn choose(cx: &mut GoalCx, kind: &str, skipped: &mut Vec<BlockPos>) -> Option<BlockPos> {
    if let Some(p) = level_neighbour(cx, kind, skipped) {
        return Some(p);
    }
    let feet = cx.me_pos().block();
    for _ in 0..MAX_REJECTS {
        let mut exclude = skipped.clone();
        exclude.push(feet.below());
        let p = cx.world.nearest_block(kind, feet, SEARCH_RADIUS, &exclude)?;
        if safe(cx, p) {
            return Some(p);
        }
        skipped.push(p);
    }
    None
}

/// `p` is level with the body (feet or head height) in a neighbouring column.
pub fn beside(feet: BlockPos, p: BlockPos) -> bool {
    let (dx, dz) = ((p.x - feet.x).abs(), (p.z - feet.z).abs());
    dx + dz == 1 && (p.y == feet.y || p.y == feet.y + 1)
}
