//! Footing probes: how far the ground holds along a heading, and the nearest ledge an
//! airborne player can still land on. Used by the combat footing (do not fight with the
//! back to a drop), the retreat heading, and the knockback recovery (R15).

use crate::math::{BlockPos, Vec3};
use crate::snapshot::{BlockKind, LocalBlocks};

/// Deepest drop that costs no health (vanilla fall damage starts beyond 3 blocks).
pub const SAFE_FALL: i32 = 3;
const STEP: f64 = 0.25;
/// Player hitbox half-width: the box keeps support until its centre is this far past the
/// edge.
const HALF_WIDTH: f64 = 0.3;

/// What stopped a clearance probe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bound {
    /// Nothing within the probe length.
    Open,
    /// A block at feet or head height; it also stops knockback.
    Wall,
    /// An unsafe drop: deeper than `SAFE_FALL`, into lava, or into unsensed space.
    Drop,
}

/// Stepping with the feet into `cell` drops the player unsafely. Water and other cushions
/// are safe; an `Unknown` floor counts as unsafe.
pub fn unsafe_drop(blocks: &LocalBlocks, cell: BlockPos) -> bool {
    let floor = blocks.get(cell.below());
    if floor.supports() || floor.negates_fall() {
        return false;
    }
    match blocks.floor_below(cell.below(), SAFE_FALL + 1) {
        Some((_, k)) if k.negates_fall() => false,
        Some((p, k)) if k.supports() => cell.y - (p.y + 1) > SAFE_FALL,
        _ => true,
    }
}

/// Distance the player's centre can travel from `from` along `heading` before reaching a
/// wall or an unsafe drop, capped at `max`. The drop distance includes the half-width
/// overhang the box keeps support for.
pub fn clearance(blocks: &LocalBlocks, from: Vec3, heading: Vec3, max: f64) -> (f64, Bound) {
    let h = heading.horizontal().normalized();
    if h.len() < 0.5 {
        return (max, Bound::Open);
    }
    let feet_y = from.block().y;
    let mut d = STEP;
    while d <= max + 1e-9 {
        let p = from + h * d;
        let cell = BlockPos::new(p.x.floor() as i32, feet_y, p.z.floor() as i32);
        if !blocks.get(cell).passable() || !blocks.get(cell.above()).passable() {
            return (d - STEP, Bound::Wall);
        }
        if blocks.get(cell).contact_hazard() || unsafe_drop(blocks, cell) {
            return ((d - STEP + HALF_WIDTH).min(max), Bound::Drop);
        }
        d += STEP;
    }
    (max, Bound::Open)
}

/// Room behind the player along a knockback push away from `threat`. Walls stop knockback,
/// so only a drop limits it.
pub fn knockback_room(blocks: &LocalBlocks, pos: Vec3, threat: Vec3, max: f64) -> f64 {
    let push = (pos - threat).horizontal();
    match clearance(blocks, pos, push, max) {
        (d, Bound::Drop) => d,
        _ => max,
    }
}

/// A ledge an airborne player could steer back onto.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ledge {
    /// Standable cell (feet position) on the ledge.
    pub cell: BlockPos,
    /// Unit horizontal heading from the player toward the nearest point of that cell.
    pub heading: Vec3,
    /// Horizontal distance the centre still has to cover before the box overlaps the cell.
    pub gap: f64,
}

/// Nearest standable cell within `radius` blocks whose surface is at or below the feet and
/// at most `SAFE_FALL` below them, excluding the cell the player is already over.
pub fn nearest_ledge(blocks: &LocalBlocks, pos: Vec3, radius: i32) -> Option<Ledge> {
    let feet = pos.block();
    let mut best: Option<Ledge> = None;
    for dy in (-SAFE_FALL - 1)..=1 {
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let cell = feet.offset(dx, dy, dz);
                let surface = cell.y as f64;
                if surface > pos.y + 1e-3 || pos.y - surface > SAFE_FALL as f64 {
                    continue;
                }
                if !blocks.standable(cell) {
                    continue;
                }
                let near = Vec3::new(
                    pos.x.clamp(cell.x as f64, cell.x as f64 + 1.0),
                    pos.y,
                    pos.z.clamp(cell.z as f64, cell.z as f64 + 1.0),
                );
                let to = (near - pos).horizontal();
                if to.len() < 1e-6 {
                    continue;
                }
                let gap = (to.len() - HALF_WIDTH).max(0.0);
                if best.is_none_or(|b| gap < b.gap) {
                    best = Some(Ledge {
                        cell,
                        heading: to.normalized(),
                        gap,
                    });
                }
            }
        }
    }
    best
}

/// Nearest standable cell beside water within `radius`, at the feet level or one above
/// (where a swimmer can climb out), by horizontal distance.
pub fn nearest_shore(blocks: &LocalBlocks, pos: Vec3, radius: i32) -> Option<BlockPos> {
    let feet = pos.block();
    let mut best: Option<(f64, BlockPos)> = None;
    for dy in [0, 1, -1] {
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let cell = feet.offset(dx, dy, dz);
                if !blocks.standable(cell) {
                    continue;
                }
                let wet = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(x, z)| {
                    let n = cell.offset(x, 0, z);
                    blocks.get(n) == BlockKind::Water || blocks.get(n.below()) == BlockKind::Water
                });
                if !wet {
                    continue;
                }
                let d = (cell.feet_centre() - pos).horizontal().len();
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, cell));
                }
            }
        }
    }
    best.map(|(_, p)| p)
}
