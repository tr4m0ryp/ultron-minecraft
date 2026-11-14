//! Edge combat and airborne recovery: combat footing, edge-aware retreat, R15 knockback
//! recovery, R6 fall arrest (block clutch and water), and C3 swimming.

mod common;

use common::*;
use reflex_core::behaviours::common::{clearance, knockback_room, open_retreat, Bound};
use reflex_core::behaviours::defence::{footing, melee_commands};
use reflex_core::behaviours::hazard::{FallArrest, KnockbackRecovery};
use reflex_core::behaviours::locomotion::Swim;
use reflex_core::channels::{GazeCmd, MainCmd};
use reflex_core::math::yaw_pitch_to;
use reflex_core::snapshot::{BlockKind, EntityKind, ItemKind};
use reflex_core::{Behaviour, BlockPos, Priority, Vec3};

/// Flat floor that ends at x = 3 with nothing below down to the void.
fn platform_edge() -> reflex_core::Snapshot {
    let mut s = flat();
    cliff(&mut s, 3, None);
    s
}

#[test]
fn clearance_sees_drop_and_wall() {
    let mut s = platform_edge();
    let (d, b) = clearance(&s.blocks, Vec3::new(1.5, 64.0, 0.5), Vec3::new(1.0, 0.0, 0.0), 3.0);
    assert_eq!(b, Bound::Drop);
    assert!((1.4..1.7).contains(&d), "edge 1.5 ahead plus overhang, got {d}");
    s.blocks.set(BlockPos::new(-2, 64, 0), BlockKind::Solid);
    let (_, b) = clearance(&s.blocks, Vec3::new(0.5, 64.0, 0.5), Vec3::new(-1.0, 0.0, 0.0), 3.0);
    assert_eq!(b, Bound::Wall);
    assert_eq!(
        knockback_room(&s.blocks, Vec3::new(0.5, 64.0, 0.5), Vec3::new(3.5, 64.0, 0.5), 3.0),
        3.0,
        "a wall behind stops knockback"
    );
}

/// The old retreat only checked the next cell, ran east onto the edge and was then
/// cornered there with its back to the drop.
#[test]
fn retreat_turns_along_the_edge_instead_of_onto_it() {
    let mut s = platform_edge();
    s.me.pos = Vec3::new(1.5, 64.0, 0.5);
    let h = open_retreat(&s, Vec3::new(-2.5, 64.0, 0.5)).expect("room to the sides");
    assert!(h.x < 0.3, "not toward the edge: {h:?}");
}

#[test]
fn footing_steps_past_the_foe_when_back_is_to_the_edge() {
    let mut s = platform_edge();
    s.me.pos = Vec3::new(2.5, 64.0, 0.5);
    let foe = Vec3::new(0.5, 64.0, 0.5);
    let step = footing(&s, foe).expect("back to the edge");
    assert!(step.heading.x < 0.0, "away from the drop: {step:?}");

    give(&mut s, 0, ItemKind::Sword, 1);
    let z = mob(7, EntityKind::Zombie, 0.5, 0.5);
    let cmds = melee_commands(&s, &z);
    assert!(cmds.legs.unwrap().heading.x < 0.0);
    assert_eq!(cmds.main, Some(MainCmd::Attack(7)));

    s.me.pos = Vec3::new(-2.5, 64.0, 0.5);
    assert!(footing(&s, Vec3::new(-4.5, 64.0, 0.5)).is_none(), "room behind");
}

#[test]
fn r15_steers_back_to_the_ledge_when_knocked_off() {
    let mut s = platform_edge();
    s.me.pos = Vec3::new(3.6, 64.3, 0.5);
    s.me.vel = Vec3::new(0.3, 0.2, 0.0);
    s.me.on_ground = false;
    s.me.health = 7.0;
    let bid = eval(&mut KnockbackRecovery, &s, &knobs())
        .0
        .expect("over the void beside the ledge");
    assert_eq!(bid.priority, Priority::P0);
    assert!(bid.commands.legs.unwrap().heading.x < -0.9);
    assert!(bid.degraded.unwrap().gaze.is_none(), "legs-only fallback");

    s.me.pos = Vec3::new(1.5, 64.8, 0.5);
    assert!(
        eval(&mut KnockbackRecovery, &s, &knobs()).0.is_none(),
        "an ordinary jump over the platform"
    );
}

#[test]
fn r15_is_p1_for_a_survivable_drop() {
    let mut s = flat();
    cliff(&mut s, 3, Some(6));
    s.me.pos = Vec3::new(3.5, 64.1, 0.5);
    s.me.vel = Vec3::new(0.2, 0.0, 0.0);
    s.me.on_ground = false;
    let bid = eval(&mut KnockbackRecovery, &s, &knobs()).0.expect("harmful");
    assert_eq!(bid.priority, Priority::P1);
}

#[test]
fn r6_block_clutch_places_on_the_ledge_face_once_aimed() {
    let mut s = platform_edge();
    s.me.pos = Vec3::new(3.5, 64.5, 0.5);
    s.me.on_ground = false;
    s.me.health = 7.0;
    give(&mut s, 2, ItemKind::ThrowawayBlock, 16);
    let mut r6 = FallArrest::default();
    let bid = eval(&mut r6, &s, &knobs()).0.expect("fires");
    assert_eq!(bid.commands.main, Some(MainCmd::SelectSlot(2)));

    s.me.selected_slot = 2;
    let bid = eval(&mut r6, &s, &knobs()).0.expect("fires");
    assert_eq!(bid.priority, Priority::P0);
    assert_eq!(bid.commands.main, None, "not aimed yet");
    let face = Vec3::new(3.0, 63.5, 0.5);
    let (yaw, pitch) = yaw_pitch_to(s.me.eye(), face);
    s.me.yaw = yaw;
    s.me.pitch = pitch;
    let bid = eval(&mut r6, &s, &knobs()).0.expect("fires");
    assert_eq!(
        bid.commands.main,
        Some(MainCmd::PlaceOn(BlockPos::new(2, 63, 0)))
    );
}

fn high_fall() -> reflex_core::Snapshot {
    let mut s = flat();
    s.blocks.fill(
        BlockPos::new(-3, 64, -3),
        BlockPos::new(3, 100, 3),
        BlockKind::Air,
    );
    s.me.pos = Vec3::new(0.5, 90.0, 0.5);
    s.me.vel = Vec3::new(0.0, -1.5, 0.0);
    s.me.on_ground = false;
    give(&mut s, 8, ItemKind::WaterBucket, 1);
    s
}

#[test]
fn r6_water_preselects_aims_down_and_places_once_at_the_lead() {
    let mut s = high_fall();
    let mut r6 = FallArrest::default();
    let bid = eval(&mut r6, &s, &knobs()).0.expect("lethal fall");
    assert_eq!(bid.commands.main, Some(MainCmd::SelectSlot(8)));
    assert!(matches!(bid.commands.gaze, Some(GazeCmd::Angles { pitch, .. }) if pitch == 90.0));

    s.me.selected_slot = 8;
    let bid = eval(&mut r6, &s, &knobs()).0.expect("falling");
    assert_eq!(bid.commands.main, None, "too early");

    s.me.pos.y = 66.5;
    let (bid, events) = eval(&mut r6, &s, &knobs());
    assert_eq!(bid.unwrap().commands.main, Some(MainCmd::Use));
    assert!(events.iter().any(|e| e.kind == "water_place"));
    r6.granted(true);
    s.me.pos.y = 65.5;
    let bid = eval(&mut r6, &s, &knobs()).0.expect("still falling");
    assert_eq!(bid.commands.main, None, "one shot: a second use would scoop it");
}

#[test]
fn r6_retrieves_the_water_after_landing() {
    let mut s = high_fall();
    s.me.selected_slot = 8;
    s.me.pos.y = 66.5;
    let mut r6 = FallArrest::default();
    let _ = eval(&mut r6, &s, &knobs());
    r6.granted(true);
    s.me.pos.y = 64.0;
    s.me.vel = Vec3::ZERO;
    s.me.on_ground = true;
    s.me.in_water = true;
    s.tick += 3;
    give(&mut s, 8, ItemKind::EmptyBucket, 1);
    s.blocks.set(BlockPos::new(0, 64, 0), BlockKind::Water);
    let bid = eval(&mut r6, &s, &knobs()).0.expect("retrieve");
    assert_eq!(bid.note, "water_retrieve");
    assert_eq!(bid.commands.main, Some(MainCmd::Use));
}

#[test]
fn r6_quiet_for_a_survivable_fall() {
    let mut s = high_fall();
    s.me.pos.y = 70.0;
    s.me.vel = Vec3::new(0.0, -0.3, 0.0);
    assert!(eval(&mut FallArrest::default(), &s, &knobs()).0.is_none());
}

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
