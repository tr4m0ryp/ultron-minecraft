//! C3 swim: tread water at the surface and stroke to the nearest shore in view.

mod common;

use common::*;
use reflex_core::behaviours::locomotion::Swim;
use reflex_core::snapshot::BlockKind;
use reflex_core::{BlockPos, Priority, Vec3};

/// Pool spanning x -6..3, z -3..3, floor at y 59; dry stone shore beyond x = 3.
fn pool() -> reflex_core::Snapshot {
    let mut s = flat();
    s.blocks.fill(
        BlockPos::new(-6, 60, -3),
        BlockPos::new(3, 63, 3),
        BlockKind::Water,
    );
    s.blocks.fill(
        BlockPos::new(-6, 59, -3),
        BlockPos::new(3, 59, 3),
        BlockKind::Solid,
    );
    s.blocks.fill(
        BlockPos::new(-6, 64, -3),
        BlockPos::new(3, 64, 3),
        BlockKind::Air,
    );
    s.me.pos = Vec3::new(2.5, 63.3, 0.5);
    s.me.on_ground = false;
    s.me.in_water = true;
    s
}

#[test]
fn c3_swims_to_the_nearest_shore_holding_jump() {
    let s = pool();
    let bid = eval(&mut Swim, &s, &knobs()).0.expect("in deep water");
    let legs = bid.commands.legs.unwrap();
    assert!(legs.jump);
    assert!(legs.heading.x > 0.6, "east shore is nearest: {legs:?}");
    assert_eq!(bid.priority, Priority::P4);
}

#[test]
fn c3_treads_water_with_no_shore_in_view() {
    let mut s = pool();
    s.blocks.fill(
        BlockPos::new(-12, 60, -12),
        BlockPos::new(12, 63, 12),
        BlockKind::Water,
    );
    let bid = eval(&mut Swim, &s, &knobs()).0.expect("tread");
    let legs = bid.commands.legs.unwrap();
    assert!(legs.jump && legs.heading.len() < 1e-6);
    assert_eq!(bid.note, "tread_water");
}

#[test]
fn c3_stands_down_on_the_bottom_or_for_a_goal() {
    let mut s = pool();
    s.me.on_ground = true;
    assert!(eval(&mut Swim, &s, &knobs()).0.is_none(), "wading");
    s.me.on_ground = false;
    let mut k = knobs();
    k.set_goal("cross", 0.5, Some(Vec3::new(-5.0, 64.0, 0.5)), false);
    assert!(eval(&mut Swim, &s, &k).0.is_none());
}
