//! H1 with nothing to eat: thought hears it as a wake with the way out.

mod common;

use common::*;
use reflex_core::behaviours::homeostasis::Hunger;
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
