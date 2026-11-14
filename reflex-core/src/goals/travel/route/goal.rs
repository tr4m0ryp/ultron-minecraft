//! Where a route may end, and how far a cell looks from there.

use super::cell::{Cell, Terrain};
use super::moves::WALK;
use crate::math::{BlockPos, Vec3};

/// Cells above the head scanned for open sky.
pub const SKY_SCAN: i32 = 48;
/// Estimated ticks per solid block on the straight line to the goal, when digging is on:
/// below the true cost of a dug block, so the search stays close to optimal, but enough
/// to keep it from flooding solid ground.
const SOLID_EST: f32 = 25.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RouteGoal {
    /// Stand within `radius` of `centre` (measured from the body's middle).
    Near { centre: Vec3, radius: f64 },
    /// Stand under open sky with room to walk off; `hint` (the last place known to have
    /// it) steers the search.
    Sky { hint: BlockPos },
    /// Stand level with `block` (at feet or head height) in a cardinal neighbour column,
    /// where a player mines it from the side.
    Beside { block: BlockPos },
}

impl RouteGoal {
    pub fn reached(&self, t: &dyn Terrain, n: BlockPos) -> bool {
        match *self {
            RouteGoal::Near { centre, radius } => mid(n).dist(centre) <= radius.max(0.5),
            RouteGoal::Sky { .. } => escaped(t, n),
            RouteGoal::Beside { block } => {
                let (dx, dz) = ((block.x - n.x).abs(), (block.z - n.z).abs());
                dx + dz == 1 && (block.y == n.y || block.y == n.y + 1)
            }
        }
    }

    /// Heuristic ticks from `n`: walking distance, plus solid blocks on the straight line
    /// when the route may dig.
    pub fn estimate(&self, t: &dyn Terrain, n: BlockPos, dig: bool) -> f32 {
        let (d, aim) = match *self {
            RouteGoal::Near { centre, radius } => ((mid(n).dist(centre) - radius).max(0.0), centre),
            RouteGoal::Sky { hint } => {
                let up = f64::from((hint.y - n.y).max(0));
                let side = (n.feet_centre() - hint.feet_centre()).horizontal().len();
                (up + side * 0.3, hint.feet_centre())
            }
            RouteGoal::Beside { block } => {
                let c = block.feet_centre();
                ((n.feet_centre().dist(c) - 1.0).max(0.0), c)
            }
        };
        let solid = if dig { solid_on_line(t, mid(n), aim) } else { 0 };
        d as f32 * WALK + solid as f32 * SOLID_EST
    }
}

fn mid(n: BlockPos) -> Vec3 {
    n.feet_centre() + Vec3::new(0.0, 0.5, 0.0)
}

/// Solid cells met stepping one block at a time from `a` toward `b` (the end excluded).
fn solid_on_line(t: &dyn Terrain, a: Vec3, b: Vec3) -> u32 {
    let d = b - a;
    let steps = d.len().floor() as i32;
    (1..steps)
        .filter(|&i| t.cell((a + d * (f64::from(i) / d.len())).block()).is_solid())
        .count() as u32
}

/// Nothing but open air (or the world's top) above the head.
pub fn open_sky(t: &dyn Terrain, n: BlockPos) -> bool {
    (2..=SKY_SCAN).all(|dy| matches!(t.cell(n.offset(0, dy, 0)), Cell::Open | Cell::Unknown))
}

/// Out in the open: sky overhead and a neighbouring column the body can walk into that
/// has sky too, so the bottom of a shaft does not count.
pub fn escaped(t: &dyn Terrain, n: BlockPos) -> bool {
    open_sky(t, n)
        && [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dz)| {
            let c = n.offset(dx, 0, dz);
            t.cell(c).is_open() && t.cell(c.above()).is_open() && open_sky(t, c)
        })
}
