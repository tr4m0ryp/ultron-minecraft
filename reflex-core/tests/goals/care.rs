use crate::sim::Sim;
use reflex_core::snapshot::{EntityKind, ItemKind};
use reflex_core::BlockPos;

fn bread() -> ItemKind {
    ItemKind::Food {
        nutrition: 5,
        saturation: 6.0,
    }
}

#[test]
fn sleep_walks_to_the_bed_at_night() {
    let mut s = Sim::new();
    s.snap.is_night = true;
    s.block(BlockPos::new(7, 64, -3), "red_bed");
    s.goal("sleep_in_bed", true);
    assert!(s.run_until(200, |s| s.has_event("goal_done")));
    assert!(s.world.state().sleeping);
}

#[test]
fn sleep_refuses_by_day_without_bed_and_waits_out_monsters() {
    // By day the body still walks over and clicks the bed once (that sets the spawn point).
    let mut s = Sim::new();
    s.block(BlockPos::new(7, 64, -3), "red_bed");
    s.goal("sleep", true);
    assert!(s.run_until(200, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("only sleep at night"));
    assert_eq!(s.world.state().bed_clicks, 1);
    assert!(!s.world.state().sleeping);

    let mut s = Sim::new();
    s.snap.is_night = true;
    s.goal("sleep", true);
    assert!(s.run_until(10, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("no bed"));

    let mut s = Sim::new();
    s.snap.is_night = true;
    s.block(BlockPos::new(2, 64, 0), "white_bed");
    s.mob(5, EntityKind::Witch, -6.5, 0.5);
    s.goal("sleep", true);
    assert!(s.run_until(60, |s| s.has_event("goal_blocked")));
    assert!(s.goal_text("goal_blocked").contains("monsters nearby"));
    assert!(!s.world.state().sleeping);
    s.snap.entities.clear();
    assert!(s.run_until(60, |s| s.has_event("goal_done")));
}

#[test]
fn eat_now_selects_food_and_eats() {
    let mut s = Sim::new();
    s.snap.me.food = 16;
    s.hotbar(5, bread(), 3);
    s.goal("eat_now", true);
    assert!(s.run_until(80, |s| s.has_event("goal_done")));
    assert_eq!(s.snap.me.food, 20);
    assert_eq!(s.snap.me.hotbar[5].unwrap().count, 2);
}

#[test]
fn eat_now_fails_without_food_or_appetite() {
    let mut s = Sim::new();
    s.snap.me.food = 10;
    s.goal("eat", true);
    assert!(s.run_until(10, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("no food"));

    let mut s = Sim::new();
    s.hotbar(5, bread(), 3);
    s.goal("eat", true);
    assert!(s.run_until(10, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("not hungry"));
}
