//! Blocks that take two cells: a bed's head goes one block along the player's facing (the
//! server reads the yaw at the click, rounded to the nearest side), a door's top half goes
//! above. Live, a bed set against a shelter's back wall is refused when the facing points
//! into the wall, and a click sent before the turn reaches the server uses the yaw of the
//! walk that brought the body there.

use super::stand::face_point;
use crate::math::{BlockPos, Vec3};

/// Ticks the aim is held before the first click of a two-cell block, so the server has the
/// new yaw when the click lands.
pub const AIM_SETTLE: u32 = 2;

/// What a placement needs beyond its own cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extra {
    /// One-cell block.
    Nothing,
    /// This cell must be open as well.
    Cell(BlockPos),
    /// The facing is too close to a diagonal to say where the head goes.
    Unclear,
}

pub fn two_cells(block: &str) -> bool {
    block.ends_with("_bed") || block.ends_with("_door")
}

/// The side the server rounds a look from `eye` toward `target` to, as an (x, z) step.
/// `margin` is how much the main axis must outweigh the other (1.0 accepts any look that is
/// not exactly diagonal); `None` for a look too close to a diagonal or straight down.
pub fn facing(eye: Vec3, target: Vec3, margin: f64) -> Option<(i32, i32)> {
    let (dx, dz) = (target.x - eye.x, target.z - eye.z);
    let (ax, az) = (dx.abs(), dz.abs());
    if ax.max(az) < 1e-3 {
        return None;
    }
    if ax > az * margin && ax > az {
        Some((dx.signum() as i32, 0))
    } else if az > ax * margin && az > ax {
        Some((0, dz.signum() as i32))
    } else {
        None
    }
}

/// The second cell a `block` placed into `cell` from `eye`, clicking the face of
/// `support`, fills.
pub fn extra_cell(block: &str, eye: Vec3, cell: BlockPos, support: BlockPos, margin: f64) -> Extra {
    if block.ends_with("_door") {
        return Extra::Cell(cell.above());
    }
    if !block.ends_with("_bed") {
        return Extra::Nothing;
    }
    match facing(eye, face_point(cell, support), margin) {
        Some((x, z)) => Extra::Cell(cell.offset(x, 0, z)),
        None => Extra::Unclear,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eye(x: f64, z: f64) -> Vec3 {
        Vec3::new(x, 65.62, z)
    }

    #[test]
    fn a_bed_head_goes_along_the_facing_and_a_door_top_above() {
        let cell = BlockPos::new(5, 64, 5);
        let floor = cell.below();
        // Standing west of the cell and looking east: the head goes east.
        let e = eye(3.5, 5.5);
        assert_eq!(
            extra_cell("white_bed", e, cell, floor, 1.5),
            Extra::Cell(BlockPos::new(6, 64, 5))
        );
        // Standing north (smaller z) and looking south: the head goes south.
        assert_eq!(
            extra_cell("red_bed", eye(5.5, 3.5), cell, floor, 1.5),
            Extra::Cell(BlockPos::new(5, 64, 6))
        );
        assert_eq!(
            extra_cell("oak_door", e, cell, floor, 1.5),
            Extra::Cell(cell.above())
        );
        assert_eq!(extra_cell("stone", e, cell, floor, 1.5), Extra::Nothing);
    }

    #[test]
    fn a_diagonal_look_is_unclear_when_planning_but_rounds_at_the_click() {
        let cell = BlockPos::new(5, 64, 5);
        let floor = cell.below();
        // One block off on both axes: exactly diagonal.
        assert_eq!(
            extra_cell("white_bed", eye(4.5, 4.5), cell, floor, 1.5),
            Extra::Unclear
        );
        // Slightly more east than south: a planner wants a clear margin, the click rounds.
        let e = eye(3.9, 4.6);
        assert_eq!(extra_cell("white_bed", e, cell, floor, 1.5), Extra::Unclear);
        assert_eq!(
            extra_cell("white_bed", e, cell, floor, 1.0),
            Extra::Cell(BlockPos::new(6, 64, 5))
        );
        // Straight down onto the cell's floor has no facing at all.
        assert_eq!(facing(eye(5.5, 5.5), eye(5.5, 5.5), 1.0), None);
        assert!(two_cells("white_bed") && two_cells("spruce_door") && !two_cells("chest"));
    }
}
