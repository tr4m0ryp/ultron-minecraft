//! Line of sight over the sensed blocks, and the nearest spot that breaks it.

use crate::math::{BlockPos, Vec3};
use crate::snapshot::{BlockKind, LocalBlocks, Snapshot};

/// Player eye height above the feet.
const EYE_HEIGHT: f64 = 1.62;

/// A block whose collision shape stops a mob's sight line and an arrow. Unknown blocks
/// (outside the sensing radius) count as clear, so exposure is never underestimated.
fn blocks_sight(k: BlockKind) -> bool {
    k.supports() && k != BlockKind::Cushion
}

/// True when no sight-blocking block lies on the segment from `a` to `b`. The cells holding
/// the two end points are not tested.
pub fn line_of_sight(blocks: &LocalBlocks, a: Vec3, b: Vec3) -> bool {
    ray_clear(a, b, |cell| !blocks_sight(blocks.get(cell)))
}

/// Walks the cells the segment from `a` to `b` crosses (voxel walk, Amanatides and Woo) and
/// asks `open` about each one strictly between the cells of the two end points; false at
/// the first that is not open.
pub fn ray_clear(a: Vec3, b: Vec3, mut open: impl FnMut(BlockPos) -> bool) -> bool {
    let d = b - a;
    let len = d.len();
    if len < 1e-9 {
        return true;
    }
    let (start, end) = (a.block(), b.block());
    let mut cell = start;
    let axis = |p: f64, v: f64| -> (i32, f64, f64) {
        if v > 0.0 {
            (1, (p.floor() + 1.0 - p) / v, 1.0 / v)
        } else if v < 0.0 {
            (-1, (p - p.floor()) / -v, -1.0 / v)
        } else {
            (0, f64::INFINITY, f64::INFINITY)
        }
    };
    let (sx, mut tx, dx) = axis(a.x, d.x);
    let (sy, mut ty, dy) = axis(a.y, d.y);
    let (sz, mut tz, dz) = axis(a.z, d.z);
    // At most one step per crossed cell boundary on each axis.
    let steps = (end.x - start.x).abs() + (end.y - start.y).abs() + (end.z - start.z).abs();
    for _ in 0..steps {
        if tx <= ty && tx <= tz {
            cell.x += sx;
            tx += dx;
        } else if ty <= tz {
            cell.y += sy;
            ty += dy;
        } else {
            cell.z += sz;
            tz += dz;
        }
        if cell == end {
            return true;
        }
        if !open(cell) {
            return false;
        }
    }
    true
}

/// A mob with its eye at `from` can see a player whose feet are at `feet` (vanilla tests
/// eye to eye).
pub fn sees(blocks: &LocalBlocks, from: Vec3, feet: Vec3) -> bool {
    line_of_sight(blocks, from, feet + Vec3::new(0.0, EYE_HEIGHT, 0.0))
}

/// Cover is searched this far around the feet.
pub const COVER_RADIUS: i32 = 5;

/// Nearest standable cell on the feet level within `COVER_RADIUS` that none of `eyes` can
/// see, reachable by a straight walk. `None` in the open.
pub fn cover_spot(snap: &Snapshot, eyes: &[Vec3]) -> Option<BlockPos> {
    let me = snap.me.pos;
    let feet = me.block();
    let mut best: Option<(f64, BlockPos)> = None;
    for dx in -COVER_RADIUS..=COVER_RADIUS {
        for dz in -COVER_RADIUS..=COVER_RADIUS {
            let p = feet.offset(dx, 0, dz);
            let d = p.feet_centre().dist(me);
            if best.is_some_and(|(b, _)| d >= b) || !snap.blocks.standable(p) {
                continue;
            }
            let hidden = eyes
                .iter()
                .all(|&e| !sees(&snap.blocks, e, p.feet_centre()));
            if hidden && walkable(&snap.blocks, me, p) {
                best = Some((d, p));
            }
        }
    }
    best.map(|(_, p)| p)
}

/// Every cell a straight walk from `from` to the cell `to` passes keeps the body clear.
fn walkable(blocks: &LocalBlocks, from: Vec3, to: BlockPos) -> bool {
    let target = to.feet_centre();
    let n = (target.dist(from) / 0.4).ceil().max(1.0) as i32;
    (1..=n).all(|i| {
        let p = from + (target - from) * (i as f64 / n as f64);
        let c = BlockPos::new(p.x.floor() as i32, to.y, p.z.floor() as i32);
        let body = |q: BlockPos| blocks.get(q).passable() && !blocks.get(q).contact_hazard();
        body(c) && body(c.above()) && !blocks.get(c.below()).passable()
    })
}
