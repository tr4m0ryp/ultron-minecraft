//! Archers: line of sight and cover, the D3 stance, R14 shield hold, and meal gating.
mod common;

use common::*;
use reflex_core::behaviours::common::{cover_spot, line_of_sight};
use reflex_core::behaviours::defence::*;
use reflex_core::behaviours::homeostasis::{Health, Hunger};
use reflex_core::channels::{GazeCmd, MainCmd, OffCmd, Speed};
use reflex_core::knobs::GainName;
use reflex_core::snapshot::{BlockKind, Entity, EntityKind, ItemKind, ItemStack, Snapshot};
use reflex_core::{BlockPos, Vec3};

fn archer(id: u32, x: f64, z: f64, draw: Option<u16>) -> Entity {
    let mut e = mob(id, EntityKind::Skeleton, x, z);
    if let Some(t) = draw {
        e.state.drawing_bow = true;
        e.state.draw_ticks = t;
    }
    e
}

fn wall(s: &mut Snapshot, a: (i32, i32), b: (i32, i32)) {
    let (lo, hi) = (BlockPos::new(a.0, 64, a.1), BlockPos::new(b.0, 66, b.1));
    s.blocks.fill(lo, hi, BlockKind::Solid);
}

fn shield(s: &mut Snapshot) {
    s.me.offhand = Some(ItemStack::new(ItemKind::Shield, 1));
}

#[test]
fn sight_is_blocked_by_walls_not_by_air_or_unknown() {
    let mut s = flat();
    let (a, b) = (Vec3::new(0.5, 65.6, 0.5), Vec3::new(0.5, 65.6, 10.5));
    assert!(line_of_sight(&s.blocks, a, b));
    assert!(
        line_of_sight(&s.blocks, a, Vec3::new(0.5, 65.6, 30.5)),
        "unknown counts as clear"
    );
    wall(&mut s, (-2, 5), (2, 5));
    assert!(!line_of_sight(&s.blocks, a, b));
    assert!(line_of_sight(&s.blocks, a, Vec3::new(8.5, 65.6, 3.5)));
}

#[test]
fn cover_is_the_nearest_hidden_reachable_cell() {
    let mut s = flat();
    let eye = archer(1, 0.5, 10.5, None).head();
    assert_eq!(cover_spot(&s, &[eye]), None, "open ground");
    wall(&mut s, (2, 2), (3, 3));
    let spot = cover_spot(&s, &[eye]).expect("behind the pillar");
    assert!(spot.z <= 1 && spot.x >= 2, "south of the pillar: {spot:?}");
    assert!(!reflex_core::behaviours::common::sees(
        &s.blocks,
        eye,
        spot.feet_centre()
    ));
}

#[test]
fn shield_goes_up_only_when_the_arrow_is_due_and_down_to_strike() {
    let mut s = flat();
    give(&mut s, 0, ItemKind::Sword, 1);
    shield(&mut s);
    s.entities.push(archer(6, 0.5, 10.5, Some(5)));
    let bid = eval(&mut RangedCover::default(), &s, &knobs()).0.unwrap();
    assert_eq!((bid.note, bid.commands.off), ("ranged_close_in", None));
    assert_eq!(
        bid.commands.legs.unwrap().speed,
        Speed::Sprint,
        "reload: sprint in"
    );
    s.entities = vec![archer(6, 0.5, 10.5, Some(17))];
    let bid = eval(&mut RangedCover::default(), &s, &knobs()).0.unwrap();
    assert_eq!(bid.note, "ranged_shield_advance");
    assert_eq!(bid.commands.off, Some(OffCmd::RaiseShield));
    assert!(matches!(bid.commands.gaze, Some(GazeCmd::LookAt(p)) if p.z > 10.0));
    s.entities = vec![archer(6, 0.5, 2.5, Some(17))];
    let bid = eval(&mut RangedCover::default(), &s, &knobs()).0.unwrap();
    assert_eq!(
        bid.commands.main,
        Some(MainCmd::Attack(6)),
        "in reach: strike"
    );
    assert_eq!(bid.commands.off, None, "shield lowered for the strike");
}

#[test]
fn without_a_shield_the_approach_zigzags_and_holds_its_side_through_a_release() {
    let mut s = flat();
    give(&mut s, 0, ItemKind::Sword, 1);
    s.entities.push(archer(6, 0.5, 11.5, Some(16)));
    let mut d3 = RangedCover::default();
    let first = eval(&mut d3, &s, &knobs()).0.unwrap();
    assert_eq!(first.note, "ranged_zigzag");
    let h = first.commands.legs.unwrap().heading;
    assert!(h.z > 0.5 && h.x.abs() > 0.5, "toward and sideways: {h:?}");
    s.tick += 40;
    let held = eval(&mut d3, &s, &knobs()).0.unwrap();
    assert_eq!(held.commands.legs.unwrap().heading, h, "arrow due: no flip");
    s.entities = vec![archer(6, 0.5, 11.5, None)];
    s.entities[0].state.since_shot = Some(12);
    let flipped = eval(&mut d3, &s, &knobs()).0.unwrap();
    assert!(
        flipped.commands.legs.unwrap().heading.x * h.x < 0.0,
        "reload: flip"
    );
}

#[test]
fn low_health_takes_cover_or_circles_instead_of_running() {
    let mut s = flat();
    s.me.health = 5.0;
    s.entities.push(archer(6, 0.5, 10.5, Some(10)));
    let bid = eval(&mut RangedCover::default(), &s, &knobs()).0.unwrap();
    assert_eq!(bid.note, "ranged_evade");
    let legs = bid.commands.legs.unwrap();
    assert!(
        legs.heading.z.abs() < 0.1 && legs.speed == Speed::Walk,
        "sideways walk"
    );
    assert!(
        eval(&mut Health, &s, &knobs()).0.is_none(),
        "H2 does not run from archers"
    );

    wall(&mut s, (2, 2), (3, 3));
    let bid = eval(&mut RangedCover::default(), &s, &knobs()).0.unwrap();
    assert_eq!(bid.note, "ranged_take_cover");
    s.me.pos = Vec3::new(2.5, 64.0, 0.5);
    let bid = eval(&mut RangedCover::default(), &s, &knobs()).0.unwrap();
    assert_eq!(bid.note, "ranged_hold_cover");
}

#[test]
fn low_aggression_on_open_ground_still_retreats_with_the_shield_up() {
    let mut s = flat();
    shield(&mut s);
    s.entities.push(archer(6, 0.5, 10.5, Some(18)));
    let mut k = knobs();
    k.set_gain(GainName::Aggression, 0.2);
    let bid = eval(&mut RangedCover::default(), &s, &k).0.unwrap();
    assert_eq!(
        (bid.note, bid.commands.off),
        ("ranged_cover", Some(OffCmd::RaiseShield))
    );
    assert!(bid.commands.legs.unwrap().heading.z < -0.5);
}

#[test]
fn r14_keeps_a_raised_shield_up_even_when_too_late_to_raise_one() {
    let mut s = flat();
    shield(&mut s);
    let mut arrow = mob(5, EntityKind::Arrow, 0.5, 4.0);
    arrow.pos.y = 65.0;
    arrow.vel = Vec3::new(0.0, 0.0, -1.5);
    s.entities.push(arrow);
    assert_eq!(
        eval(&mut ProjectileEvade, &s, &knobs()).0.unwrap().note,
        "strafe_projectile"
    );
    s.me.shield_ticks = 3;
    let bid = eval(&mut ProjectileEvade, &s, &knobs()).0.unwrap();
    assert_eq!(
        (bid.note, bid.commands.off),
        ("shield_block", Some(OffCmd::RaiseShield))
    );
}

#[test]
fn meals_wait_for_the_reload_or_for_cover() {
    let mut s = flat();
    give(&mut s, 7, bread(), 8);
    s.me.selected_slot = 7;
    s.me.food = 5;
    s.entities.push(archer(6, 0.5, 13.5, None));
    assert!(
        eval(&mut Hunger::default(), &s, &knobs()).0.is_none(),
        "may draw now"
    );
    s.entities[0].state.since_shot = Some(12);
    let bid = eval(&mut Hunger::default(), &s, &knobs()).0.unwrap();
    assert_eq!(
        bid.commands.main,
        Some(MainCmd::Use),
        "arrow landed: 48 ticks of reload left"
    );
    s.entities[0].state.since_shot = Some(40);
    assert!(eval(&mut Hunger::default(), &s, &knobs()).0.is_none());
    wall(&mut s, (-2, 5), (2, 5));
    assert!(
        eval(&mut Hunger::default(), &s, &knobs()).0.is_some(),
        "behind the wall"
    );
    assert_eq!(ticks_to_shot(&archer(1, 0.0, 0.0, Some(15))), 5);
}

#[test]
fn the_shield_stays_up_while_the_arrow_is_in_flight() {
    let mut s = flat();
    give(&mut s, 0, ItemKind::Sword, 1);
    shield(&mut s);
    s.me.attack_ready = 0.5;
    let mut sk = archer(6, 0.5, 4.5, None);
    sk.state.since_shot = Some(1);
    s.entities.push(sk);
    let bid = eval(&mut RangedCover::default(), &s, &knobs()).0.unwrap();
    assert_eq!(
        bid.commands.off,
        Some(OffCmd::RaiseShield),
        "arrow still in the air"
    );
    s.entities[0].state.since_shot = Some(8);
    let bid = eval(&mut RangedCover::default(), &s, &knobs()).0.unwrap();
    assert_eq!(bid.commands.off, None, "landed: reload window");
}
