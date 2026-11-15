//! H1 with nothing to eat: thought hears it as a wake with the way out.

mod common;

use common::*;
use reflex_core::behaviours::homeostasis::Hunger;
use reflex_core::channels::MainCmd;
use reflex_core::snapshot::ItemKind;
use reflex_core::EventClass;

#[test]
fn hungry_with_no_food_wakes_thought_once_and_again_at_emergency() {
    let mut s = flat();
    let mut h = Hunger::default();
    s.me.food = 18;
    let (_, ev) = eval(&mut h, &s, &knobs());
    assert!(
        !ev.iter().any(|e| e.kind == "hungry_no_food"),
        "not hungry yet"
    );

    s.me.food = 12;
    let (bid, ev) = eval(&mut h, &s, &knobs());
    assert!(bid.is_none(), "nothing to eat");
    let e = ev
        .iter()
        .find(|e| e.kind == "hungry_no_food")
        .expect("told");
    assert_eq!(e.class, EventClass::Wake);
    assert!(e.text.contains("collect beef") && e.text.contains("smelt"));
    let (_, ev) = eval(&mut h, &s, &knobs());
    assert!(!ev.iter().any(|e| e.kind == "hungry_no_food"), "once");

    s.me.food = 6;
    let (_, ev) = eval(&mut h, &s, &knobs());
    assert!(
        ev.iter().any(|e| e.kind == "hungry_no_food"),
        "again at emergency"
    );

    give(&mut s, 7, bread(), 1);
    let (_, ev) = eval(&mut h, &s, &knobs());
    assert!(!ev.iter().any(|e| e.kind == "hungry_no_food"));
    s.me.hotbar[7] = None;
    let (_, ev) = eval(&mut h, &s, &knobs());
    assert!(
        ev.iter().any(|e| e.kind == "hungry_no_food"),
        "food ran out again"
    );
}

fn raw_beef() -> ItemKind {
    ItemKind::Food {
        nutrition: 3,
        saturation: 1.8,
        cookable: true,
    }
}

#[test]
fn raw_meat_waits_for_the_furnace_until_the_emergency() {
    // Live: the reflex ate the raw beef a `smelt cooked_beef 3` was set to cook.
    let mut s = flat();
    let mut h = Hunger::default();
    give(&mut s, 2, raw_beef(), 3);
    s.me.food = 10;
    let (bid, ev) = eval(&mut h, &s, &knobs());
    assert!(bid.is_none(), "peckish: the beef is kept for cooking");
    let e = ev
        .iter()
        .find(|e| e.kind == "hungry_raw_food")
        .expect("thought is told");
    assert_eq!(e.class, EventClass::Wake);
    assert!(e.text.contains("smelt cooked_beef 3"), "{}", e.text);
    assert!(!ev.iter().any(|e| e.kind == "hungry_no_food"));
    let (_, ev) = eval(&mut h, &s, &knobs());
    assert!(!ev.iter().any(|e| e.kind == "hungry_raw_food"), "once");

    s.me.food = 6;
    let (bid, ev) = eval(&mut h, &s, &knobs());
    assert!(bid.is_some(), "emergency: eats it raw");
    assert!(!ev.iter().any(|e| e.kind == "hungry_raw_food"));
}

#[test]
fn raw_meals_at_the_emergency_do_not_wake_thought_again() {
    // Live: every raw meal at food 6 lifted the bar to 9 and woke thought anew.
    let mut s = flat();
    let mut h = Hunger::default();
    give(&mut s, 2, raw_beef(), 8);
    let told = |h: &mut Hunger, s: &reflex_core::Snapshot| -> Vec<EventClass> {
        let (_, ev) = eval(h, s, &knobs());
        let raw = ev.into_iter().filter(|e| e.kind == "hungry_raw_food");
        raw.map(|e| e.class).collect()
    };
    s.me.food = 10;
    assert_eq!(told(&mut h, &s), [EventClass::Wake]);
    for meal in 0..6 {
        s.tick += 200;
        s.me.food = 6;
        assert!(told(&mut h, &s).is_empty(), "meal {meal}: emergency");
        s.me.food = 9;
        assert!(told(&mut h, &s).is_empty(), "meal {meal}: same spell");
    }
    // Fed, then hungry again soon after: a digest line, not a wake.
    s.me.food = 20;
    assert!(told(&mut h, &s).is_empty());
    s.tick += 1000;
    s.me.food = 10;
    assert_eq!(told(&mut h, &s), [EventClass::Digest]);
    // And a wake again once the quiet time has passed.
    s.me.food = 20;
    assert!(told(&mut h, &s).is_empty());
    s.tick += reflex_core::behaviours::homeostasis::RAW_RETELL_TICKS;
    s.me.food = 10;
    assert_eq!(told(&mut h, &s), [EventClass::Wake]);
}

#[test]
fn ready_food_is_eaten_before_raw_and_raw_is_kept() {
    let mut s = flat();
    let mut h = Hunger::default();
    // Raw food that would outscore the bread on nutrition alone.
    let rich_raw = ItemKind::Food {
        nutrition: 9,
        saturation: 9.0,
        cookable: true,
    };
    give(&mut s, 2, rich_raw, 3);
    give(&mut s, 7, bread(), 1);
    s.me.selected_slot = 7;
    s.me.food = 10;
    let (bid, ev) = eval(&mut h, &s, &knobs());
    let bid = bid.expect("eats the bread");
    assert_eq!(
        bid.commands.main,
        Some(MainCmd::Use),
        "the bread in the held slot"
    );
    assert!(!ev.iter().any(|e| e.kind.starts_with("hungry_")));
}
