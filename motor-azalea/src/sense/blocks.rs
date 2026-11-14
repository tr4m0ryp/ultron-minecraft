use azalea::world::World;
use reflex_core::{math::BlockPos, snapshot::LocalBlocks};

use super::block_kind::classify;

pub const RADIUS_H: i32 = 6;
pub const BELOW: i32 = 8;
pub const ABOVE: i32 = 3;
/// Columns within this horizontal radius are also read up to `COLUMN_UP` above the feet, so
/// the surface reflex (R8 scans 16 up from the head) can see open water above.
pub const COLUMN_RADIUS: i32 = 4;
pub const COLUMN_UP: i32 = 18;

/// Fill `out` with the box around `feet`, the columns above it, and the full column under
/// the feet down to the world floor. Cells in unloaded chunks stay unset (`Unknown`).
pub fn read(world: &World, feet: BlockPos, out: &mut LocalBlocks) {
    let get = |p: BlockPos| {
        world
            .get_block_state(azalea::BlockPos::new(p.x, p.y, p.z))
            .map(classify)
    };
    for dx in -RADIUS_H..=RADIUS_H {
        for dz in -RADIUS_H..=RADIUS_H {
            for dy in -BELOW..=ABOVE {
                let p = feet.offset(dx, dy, dz);
                if let Some(k) = get(p) {
                    out.set(p, k);
                }
            }
        }
    }
    for dx in -COLUMN_RADIUS..=COLUMN_RADIUS {
        for dz in -COLUMN_RADIUS..=COLUMN_RADIUS {
            for dy in ABOVE + 1..=COLUMN_UP {
                let p = feet.offset(dx, dy, dz);
                if let Some(k) = get(p) {
                    out.set(p, k);
                }
            }
        }
    }
    let min_y = world.chunks.min_y();
    for y in (min_y..feet.y - BELOW).rev() {
        let p = BlockPos::new(feet.x, y, feet.z);
        match get(p) {
            Some(k) => out.set(p, k),
            None => break,
        }
    }
}

pub fn world_min_y(world: &World) -> i32 {
    world.chunks.min_y()
}
