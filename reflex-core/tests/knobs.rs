mod common;

use common::*;
use reflex_core::channels::{Channel, MainCmd};
use reflex_core::knobs::{Mode, Response, Scope, Trigger};
use reflex_core::snapshot::{BlockKind, Companion, EntityKind};
use reflex_core::{Aabb, BlockPos, EventClass, Executor, Vec3};

/// Bot at the cliff edge walking east toward a goal across the drop.
fn cliff_scene(depth: i32) -> (reflex_core::Snapshot, Executor) {
    let mut s = flat();
    s.me.pos = Vec3::new(1.5, 64.0, 0.5);
    cliff(&mut s, 2, Some(depth));
    let mut ex = Executor::new(BOT);
    ex.knobs.gains.leash = 0.0;
    ex.knobs
        .set_goal("build east", 0.8, Some(Vec3::new(8.5, 64.0, 0.5)), true);
    (s, ex)
}

#[test]
fn inhibition_suppresses_edge_guard_then_expires() {
    let (mut s, mut ex) = cliff_scene(5);
    let out = ex.tick(&s);
    assert!(
        out.sneak && out.forward == 0,
        "R5 stops at a 5-block drop by default"
    );

    let region = Aabb {
        min: Vec3::new(-5.0, 60.0, -5.0),
        max: Vec3::new(5.0, 70.0, 5.0),
    };
    ex.knobs
        .inhibit("R5", Scope::Region(region), 1.0, Some(100), s.tick);
    s.tick += 1;
    let out = ex.tick(&s);
    assert!(
        !out.sneak && out.forward == 1,
        "inhibited inside the box: walks off as the brain asked"
    );

    s.tick += 100;
    let out = ex.tick(&s);
    assert!(out.sneak && out.forward == 0, "inhibition expired");
}

#[test]
fn inhibition_is_ignored_when_the_drop_is_lethal() {
    let (mut s, mut ex) = cliff_scene(40);
    ex.knobs.inhibit("R5", Scope::Everywhere, 1.0, None, s.tick);
    s.tick += 1;
    let out = ex.tick(&s);
    assert!(
        out.sneak && out.forward == 0,
        "lethal floor overrides the inhibition"
    );
    let wake = ex.take_wake(s.tick);
    assert!(wake
        .iter()
        .any(|e| e.kind == "override" && e.subject == "R5" && e.class == EventClass::Wake));
}

#[test]
fn inhibited_p0_bid_overrides_and_reports() {
    let mut s = flat();
    s.blocks.set(BlockPos::new(0, 64, 0), BlockKind::Lava);
    let mut ex = Executor::new(BOT);
    ex.knobs.inhibit("R1", Scope::Everywhere, 1.0, None, s.tick);
    ex.tick(&s);
    assert_eq!(ex.owner(Channel::Legs), Some("R1"));
    assert!(ex.take_wake(s.tick).iter().any(|e| e.kind == "override"));
    s.tick += 1;
    let mut s2 = flat();
    s2.tick = s.tick;
    s2.blocks.set(BlockPos::new(0, 63, 0), BlockKind::Magma);
    ex.knobs.inhibit("R4", Scope::Everywhere, 1.0, None, s.tick);
    ex.tick(&s2);
    assert_ne!(
        ex.owner(Channel::Legs),
        Some("R4"),
        "P1 reflex fully inhibited"
    );
}

#[test]
fn goal_scoped_inhibition_ends_with_the_goal() {
    let (s, mut ex) = cliff_scene(5);
    let gid = ex.knobs.goal.as_ref().unwrap().id;
    ex.knobs
        .inhibit("R5", Scope::UntilGoalEnds(gid), 1.0, None, s.tick);
    assert_eq!(ex.knobs.inhibition_for("R5", s.me.pos, s.tick), 1.0);
    ex.knobs.end_goal();
    assert_eq!(ex.knobs.inhibition_for("R5", s.me.pos, s.tick), 0.0);
    assert!(ex.knobs.inhibitions.is_empty());
}

#[test]
fn armed_response_fires_at_tick_latency_without_the_brain() {
    let mut s = flat();
    s.companion = Some(Companion {
        name: "human".into(),
        entity_id: Some(99),
        pos: Vec3::new(10.5, 64.0, 0.5),
        vel: Vec3::ZERO,
        health: Some(20.0),
    });
    let mut ex = Executor::new(BOT);
    ex.knobs.gains.leash = 0.0;
    let trig = Trigger::EntityWithin {
        kind: EntityKind::Creeper,
        dist: 6.0,
        of_companion: true,
    };
    ex.knobs.arm(
        trig,
        Response::Say("creeper behind you!".into()),
        Some(6000),
        true,
        s.tick,
    );
    for _ in 0..5 {
        s.tick += 1;
        assert_eq!(ex.tick(&s).chat, None, "trigger false");
    }
    s.entities.push(mob(1, EntityKind::Creeper, 14.0, 0.5));
    s.tick += 1;
    let out = ex.tick(&s);
    assert_eq!(
        out.chat.as_deref(),
        Some("creeper behind you!"),
        "fires on the first tick the trigger holds"
    );
    assert!(ex.take_wake(s.tick).iter().any(|e| e.kind == "armed_fired"));
    assert!(ex.knobs.armed.is_empty(), "one-shot response retired");
    s.tick += 1;
    assert_eq!(ex.tick(&s).chat, None);
}

#[test]
fn armed_flee_moves_without_brain() {
    let mut s = flat();
    let mut ex = Executor::new(BOT);
    ex.knobs.arm(
        Trigger::HpBelow(10.0),
        Response::FleeFrom(EntityKind::Witch),
        None,
        false,
        s.tick,
    );
    s.entities.push(mob(4, EntityKind::Witch, 5.5, 0.5));
    s.me.health = 9.0;
    ex.tick(&s);
    assert_eq!(ex.owner(Channel::Legs), Some("ARM:1"));
}

#[test]
fn mode_and_gain_change_d2_from_retreat_to_fight() {
    let mut s = flat();
    s.entities.push(mob(3, EntityKind::Zombie, 2.5, 0.5));
    let mut ex = Executor::new(BOT);
    ex.knobs.set_mode(Mode::Retreat);
    let out = ex.tick(&s);
    assert_eq!(ex.owner(Channel::Legs), Some("D2"));
    assert!(out.main.is_none(), "retreat mode: no swing");
    ex.knobs.set_mode(Mode::Fight);
    s.tick += 1;
    let out = ex.tick(&s);
    assert_eq!(out.main, Some(MainCmd::Attack(3)));
}
