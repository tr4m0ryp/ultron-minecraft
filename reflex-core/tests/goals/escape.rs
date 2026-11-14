use crate::sim::Sim;
use reflex_core::goals::runner::ENCLOSED_TICKS;
use reflex_core::goals::NavTarget;

#[test]
fn a_shut_in_body_with_no_goal_digs_itself_out() {
    let mut s = Sim::new();
    s.world.state().no_sky = true;
    s.nav.state().shut_in = true;
    assert!(
        s.run_until(ENCLOSED_TICKS + 40, |s| s.has_event("escape")),
        "escape started without a goal"
    );
    assert!(s.run_until(100, |s| {
        matches!(s.nav.state().last, Some(NavTarget::Sky { dig: true, .. }))
    }));
    assert!(s.run_until(100, |s| s
        .events("escape")
        .iter()
        .any(|e| e.text.contains("open sky"))));
}

#[test]
fn a_walkable_way_out_or_a_goal_keeps_the_body_where_it_is() {
    // A hut with an open door: no sky overhead but a walking route out. The probe finds
    // it, and the body does not leave its shelter on its own.
    let mut s = Sim::new();
    s.world.state().no_sky = true;
    for _ in 0..(ENCLOSED_TICKS as usize + 200) {
        s.step();
    }
    assert!(!s.has_event("escape"));
    assert!(!matches!(
        s.nav.state().last,
        Some(NavTarget::Sky { dig: true, .. })
    ));
    // With a goal running nothing escapes either.
    let mut s = Sim::new();
    s.world.state().no_sky = true;
    s.nav.state().shut_in = true;
    s.goal("stay", true);
    for _ in 0..(ENCLOSED_TICKS as usize + 200) {
        s.step();
    }
    assert!(!s.has_event("escape"));
}
