//! A bed against a shelter's back wall: the head goes one block along the facing the server
//! holds at the click, so the skill must stand where that side is open and turn before it
//! clicks.

use crate::sim::Sim;
use reflex_core::channels::MainCmd;
use reflex_core::math::Vec3;
use reflex_core::BlockPos;

/// The side vanilla rounds a yaw to (0 south, 90 west, 180 north, 270 east), as (x, z).
fn side(yaw: f32) -> (i32, i32) {
    match ((yaw / 90.0).round() as i32).rem_euclid(4) {
        0 => (0, 1),
        1 => (-1, 0),
        2 => (0, -1),
        _ => (1, 0),
    }
}

fn shelter() -> Sim {
    let mut s = Sim::new();
    // 5x5 shelter around x 3..7, z -2..2: walls two high, roof, door in the west wall.
    for y in 64..=66 {
        for x in 3..=7 {
            for z in -2..=2 {
                let wall = x == 3 || x == 7 || z == -2 || z == 2;
                let door = x == 3 && z == 0 && y < 66;
                if (y == 66 || wall) && !door {
                    s.block(BlockPos::new(x, y, z), "oak_planks");
                }
            }
        }
    }
    s.item("white_bed", 1, Some(3));
    s
}

#[test]
fn a_bed_against_the_back_wall_gets_its_head_inside() {
    let mut s = shelter();
    s.snap.me.pos = Vec3::new(5.5, 64.0, 0.5);
    let foot = BlockPos::new(6, 64, 0);
    s.place_at = Some(foot);
    s.goal("place white_bed 6 64 0", true);
    let mut clicks = Vec::new();
    let mut prev_yaw = None;
    for _ in 0..300 {
        let out = s.step();
        if let Some(MainCmd::PlaceAt { cell, .. }) = out.main {
            clicks.push((cell, out.yaw, prev_yaw));
        }
        prev_yaw = Some(out.yaw);
        if s.has_event("goal_done") || s.has_event("goal_failed") {
            break;
        }
    }
    assert!(s.has_event("goal_done"), "{:?}", s.events("goal_failed"));
    assert!(!clicks.is_empty());
    for (cell, yaw, before) in clicks {
        assert_eq!(cell, foot);
        let (x, z) = side(yaw);
        let head = foot.offset(x, 0, z);
        let w = s.world.state();
        assert!(
            !w.blocks.contains_key(&head),
            "head {head:?} lands in a wall (yaw {yaw})"
        );
        assert_eq!(before, Some(yaw), "the yaw was held before the click");
    }
}

#[test]
fn no_click_while_the_body_grazes_the_bed_cell() {
    // Live: the walk ended at z 1.25 in the cell south of the foot, the hitbox 0.05 into
    // the foot cell, and the server refused eight clicks.
    let mut s = shelter();
    s.snap.me.pos = Vec3::new(6.06, 64.0, 1.25);
    let foot = BlockPos::new(6, 64, 0);
    s.place_at = Some(foot);
    s.goal("place white_bed 6 64 0", true);
    for _ in 0..300 {
        let at = s.snap.me.pos;
        let out = s.step();
        if let Some(MainCmd::PlaceAt { .. }) = out.main {
            assert!(
                at.z - 0.3 >= 1.0,
                "clicked with the body in the cell: {at:?}"
            );
        }
        if s.has_event("goal_done") || s.has_event("goal_failed") {
            break;
        }
    }
    assert!(s.has_event("goal_done"), "{:?}", s.events("goal_failed"));
}
