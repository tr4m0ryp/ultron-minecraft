use azalea::world::World;
use reflex_core::{
    math::BlockPos,
    snapshot::{BlockKind, LocalBlocks},
};

use super::block_kind::classify;

pub const RADIUS_H: i32 = 6;
pub const BELOW: i32 = 8;
pub const ABOVE: i32 = 3;

/// Fill `out` with the box around `feet` plus the full column under the feet down to the
/// world floor. Cells in unloaded chunks stay unset (`Unknown`).
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

#[allow(dead_code)]
fn _kind_is_used(_: BlockKind) {}
