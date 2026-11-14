//! Geometry helpers shared by the behaviours: escape steps, retreat headings, hazard probes.

use crate::math::{BlockPos, Vec3};
use crate::snapshot::{BlockKind, LocalBlocks, Snapshot};

/// Blocks the player's box touches: feet and head cells plus the block under the feet.
pub fn body_cells(pos: Vec3) -> [BlockPos; 3] {
    let feet = pos.block();
    [feet, feet.above(), feet.below()]
}

/// Player hitbox half-width and height (vanilla 0.6 x 1.8).
const HALF_WIDTH: f64 = 0.3;
const HEIGHT: f64 = 1.8;

/// Every block the player's 0.6 x 1.8 box overlaps. Near a block edge the box straddles up
/// to four columns, so the centre cell alone misses hazards the body is touching.
pub fn hitbox_cells(pos: Vec3) -> Vec<BlockPos> {
    let span = |lo: f64, hi: f64| (lo.floor() as i32)..=((hi - 1e-6).floor() as i32);
    let mut out = Vec::with_capacity(12);
    for x in span(pos.x - HALF_WIDTH, pos.x + HALF_WIDTH) {
        for z in span(pos.z - HALF_WIDTH, pos.z + HALF_WIDTH) {
            for y in span(pos.y, pos.y + HEIGHT) {
                out.push(BlockPos::new(x, y, z));
            }
        }
    }
    out
}

/// Nearest standable neighbour (8 directions, same level, one up, one down) for which
/// `ok` holds, closest to `pos`.
pub fn escape_step(
    blocks: &LocalBlocks,
    pos: Vec3,
    ok: impl Fn(BlockPos) -> bool,
) -> Option<BlockPos> {
    let feet = pos.block();
    let mut best: Option<(f64, BlockPos)> = None;
    for dy in [0, 1, -1] {
        for (dx, dz) in BlockPos::HORIZONTAL {
            let p = feet.offset(dx, dy, dz);
            if !blocks.standable(p) || !ok(p) || near_kind(blocks, p, BlockKind::Lava) {
                continue;
            }
            let d = p.feet_centre().dist(pos) + dy.unsigned_abs() as f64 * 0.5;
            if best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, p));
            }
        }
        if best.is_some() {
            break;
        }
    }
    best.map(|(_, p)| p)
}

/// Any horizontal neighbour of `p` (feet or head level) or the block under it is `kind`.
pub fn near_kind(blocks: &LocalBlocks, p: BlockPos, kind: BlockKind) -> bool {
    if blocks.get(p.below()) == kind || blocks.get(p) == kind {
        return true;
    }
    [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .iter()
        .any(|&(dx, dz)| blocks.get(p.offset(dx, 0, dz)) == kind)
}

/// World-frame heading directly away from `threat`; a fixed direction when on top of it.
pub fn away_from(me: Vec3, threat: Vec3) -> Vec3 {
    let d = (me - threat).horizontal();
    if d.len() < 1e-6 {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        d.normalized()
    }
}

/// Heading away from `threat` that does not lead off an unsafe drop: tries straight away,
/// then 45 and 90 degrees either side.
pub fn safe_retreat(snap: &Snapshot, threat: Vec3) -> Vec3 {
    let base = away_from(snap.me.pos, threat);
    let rot = |v: Vec3, deg: f64| {
        let (s, c) = deg.to_radians().sin_cos();
        Vec3::new(v.x * c - v.z * s, 0.0, v.x * s + v.z * c)
    };
    for deg in [0.0, 45.0, -45.0, 90.0, -90.0] {
        let h = rot(base, deg);
        let next = (snap.me.pos + h * 1.0).block();
        let floor = snap.blocks.get(next.below());
        let body_ok = snap.blocks.get(next).passable() && !snap.blocks.get(next).contact_hazard();
        if body_ok && (floor.supports() || floor == BlockKind::Water) {
            return h;
        }
    }
    base
}

pub fn heading_to(me: Vec3, target: Vec3) -> Vec3 {
    (target - me).horizontal().normalized()
}
