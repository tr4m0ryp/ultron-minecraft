mod common;

use common::*;
use reflex_core::behaviours::homeostasis::{CompanionLeash, Health, Hunger, GOAL_GRACE};
use reflex_core::channels::MainCmd;
use reflex_core::knobs::GainName;
use reflex_core::snapshot::{EntityKind, ItemKind};
use reflex_core::{EventClass, Priority};

#[test]
fn h1_hunger_eats_below_floor_only_when_safe() {
    let mut s = flat();
    give(&mut s, 7, bread(), 10);
    s.me.food = 18;
    assert!(eval(&mut Hunger::default(), &s, &knobs()).0.is_none());
    s.me.food = 10;
    let bid = eval(&mut Hunger::default(), &s, &knobs()).0.unwrap();
    assert_eq!(
        (bid.priority, bid.commands.main),
        (Priority::P3, Some(MainCmd::SelectSlot(7)))
    );
    s.me.selected_slot = 7;
    let bid = eval(&mut Hunger::default(), &s, &knobs()).0.unwrap();
    assert_eq!(bid.commands.main, Some(MainCmd::Use));
    assert!(bid.lock_ticks >= 32, "eating is atomic");

    s.entities.push(mob(1, EntityKind::Zombie, 8.0, 0.5));
    assert!(
        eval(&mut Hunger::default(), &s, &knobs()).0.is_none(),
        "hostile within 12"
    );
    s.me.food = 5;
    assert_eq!(
        eval(&mut Hunger::default(), &s, &knobs())
            .0
            .unwrap()
            .priority,
        Priority::P1,
        "emergency"
    );

    let mut k = knobs();
    k.set_gain(GainName::HungerFloor, 8.0);
    s.entities.clear();
    s.me.food = 10;
    assert!(
        eval(&mut Hunger::default(), &s, &k).0.is_none(),
        "brain lowered hunger_floor"
    );
}

#[test]
fn h1_reports_low_food_stock_once() {
    let mut s = flat();
    give(&mut s, 7, bread(), 2);
    let mut h = Hunger::default();
    let (_, ev) = eval(&mut h, &s, &knobs());
    assert!(ev.iter().any(|e| e.kind == "food_stock_low"));
    let (_, ev) = eval(&mut h, &s, &knobs());
    assert!(ev.is_empty());
}

#[test]
fn h2_health_retreats_under_floor_and_eats_golden_apple() {
    let mut s = flat();
    s.entities.push(mob(1, EntityKind::Zombie, 10.5, 0.5));
    assert!(
        eval(&mut Health, &s, &knobs()).0.is_none(),
        "full health, full food"
    );
    s.me.health = 7.0;
    let bid = eval(&mut Health, &s, &knobs()).0.unwrap();
    assert_eq!(bid.priority, Priority::P1);
    assert!(
        bid.commands.legs.unwrap().heading.x < -0.5,
        "retreats away from the zombie"
    );
    give(&mut s, 2, ItemKind::GoldenApple, 1);
    s.me.health = 5.0;
    let bid = eval(&mut Health, &s, &knobs()).0.unwrap();
    assert_eq!(bid.commands.main, Some(MainCmd::SelectSlot(2)));
}

#[test]
fn h8_leash_follow_danger_and_far() {
    let mut s = flat();
    s.companion = Some(companion_at(3.0));
    assert!(eval(&mut CompanionLeash::default(), &s, &knobs())
        .0
        .is_none());
    s.companion = Some(companion_at(10.5));
    let bid = eval(&mut CompanionLeash::default(), &s, &knobs())
        .0
        .unwrap();
    assert_eq!(bid.priority, Priority::P4);
    let mut k = knobs();
    k.set_gain(GainName::Leash, 0.0);
    assert!(
        eval(&mut CompanionLeash::default(), &s, &k).0.is_none(),
        "leash 0 disables"
    );

    s.entities.push(mob(5, EntityKind::Zombie, 12.0, 0.5));
    let (bid, ev) = eval(&mut CompanionLeash::default(), &s, &knobs());
    assert_eq!(bid.unwrap().priority, Priority::P2);
    assert!(ev
        .iter()
        .any(|e| e.kind == "companion_danger" && e.class == EventClass::Wake));

    s.entities.clear();
    s.companion = Some(companion_at(40.0));
    let (_, ev) = eval(&mut CompanionLeash::default(), &s, &knobs());
    assert!(ev.iter().any(|e| e.kind == "companion_far"));
}

#[test]
fn h8_holds_off_following_for_a_grace_after_a_goal_ends() {
    let mut s = flat();
    s.companion = Some(companion_at(20.5));
    let mut k = knobs();
    k.set_goal("mine stone", 0.5, None, true);
    let mut h8 = CompanionLeash::default();
    assert!(
        eval(&mut h8, &s, &k).0.is_none(),
        "solo goal suspends the leash"
    );

    k.end_goal();
    let t0 = s.tick;
    assert!(
        eval(&mut h8, &s, &k).0.is_none(),
        "no pull on the tick the goal ends"
    );
    s.tick = t0 + GOAL_GRACE - 1;
    assert!(eval(&mut h8, &s, &k).0.is_none(), "still inside the grace");
    s.tick = t0 + GOAL_GRACE;
    let bid = eval(&mut h8, &s, &k)
        .0
        .expect("follow resumes after the grace");
    assert_eq!(bid.note, "follow_companion");

    // Every goal end opens a fresh grace.
    k.set_goal("mine stone", 0.5, None, true);
    assert!(eval(&mut h8, &s, &k).0.is_none());
    k.end_goal();
    s.tick += 1;
    assert!(eval(&mut h8, &s, &k).0.is_none(), "second grace");

    // Danger to the companion is never held off.
    s.entities.push(mob(5, EntityKind::Zombie, 22.0, 0.5));
    let bid = eval(&mut h8, &s, &k).0.expect("danger");
    assert_eq!(bid.priority, Priority::P2);
}
