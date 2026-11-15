//! Line of sight for the hands. The server takes a click on any block within reach, also
//! one behind a wall; a player cannot make that click. Live, the body put a bed down
//! through a shelter wall and then slept in it from outside.

use super::surfaces::replaceable;
use crate::behaviours::common::ray_clear;
use crate::goals::ports::World;
use crate::math::{BlockPos, Vec3};

/// Whether an eye at `eye` has a clear line to `point`: every cell the line crosses between
/// the two holds nothing a click would land on first (air, fluids and the plants a block
/// replaces are clear; unloaded cells count as clear). Cells in `through` are not tested.
pub fn hand_sees(world: &dyn World, eye: Vec3, point: Vec3, through: &[BlockPos]) -> bool {
    ray_clear(eye, point, |c| {
        through.contains(&c) || world.block_at(c).is_none_or(|k| replaceable(&k))
    })
}
