use crate::sim::{Sim, COMPANION_ID};
use reflex_core::channels::Channel;
use reflex_core::Vec3;

#[test]
fn go_to_arrives_and_reports_done_once() {
    let mut s = Sim::new();
    s.goal("go_to 12 64 8", true);
    assert!(s.run_until(200, |s| s.has_event("goal_done")));
    assert!(s.snap.me.pos.dist(Vec3::new(12.5, 64.0, 8.5)) <= 1.2);
    assert_eq!(s.events("goal_started").len(), 1);
    assert!(
        s.events("goal_progress").len() <= 3,
        "quarter milestones only"
    );
    for _ in 0..50 {
        s.step();
    }
    assert_eq!(s.events("goal_done").len(), 1);
    assert!(
        s.exec.knobs.goal.is_none(),
        "settled: the finished goal left the knobs"
    );
}

#[test]
fn go_to_unreachable_fails_after_retries_with_reason() {
    let mut s = Sim::new();
    s.nav
        .state()
        .unreachable
        .push(reflex_core::BlockPos::new(12, 64, 8));
    s.goal("go_to 12 64 8", true);
    assert!(s.run_until(100, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("Unreachable"));
    assert_eq!(s.nav.state().requests, 3);
    assert_eq!(
        s.events("goal_blocked").len(),
        1,
        "one blocked report per reason"
    );
}

#[test]
fn go_to_yields_legs_to_the_leash_and_replans_after() {
    let mut s = Sim::new();
    s.companion_at(0.5, 0.5);
    s.goal("go_to 14 64 0", false);
    // The leash (H8, P4) pulls back beyond 6 blocks: the goal loses the legs, pauses.
    assert!(s.run_until(120, |s| s.exec.owner(Channel::Legs) == Some("H8")));
    s.step();
    assert!(
        s.nav.state().pauses >= 1,
        "planner paused while a reflex holds the legs"
    );
    let requests = s.nav.state().requests;
    // The companion walks to the destination; the leash lets go and the goal re-plans.
    s.companion_at(14.5, 0.5);
    assert!(s.run_until(300, |s| s.has_event("goal_done")));
    assert!(
        s.nav.state().requests > requests,
        "re-planned after resuming"
    );
}

#[test]
fn follow_keeps_distance_and_reports_lost_companion() {
    let mut s = Sim::new();
    s.companion_at(2.5, 0.5);
    s.goal("follow companion 3", false);
    for _ in 0..20 {
        s.step();
    }
    assert_eq!(
        s.snap.me.pos.dist(Vec3::new(0.5, 64.0, 0.5)),
        0.0,
        "already close: no move"
    );
    s.companion_at(10.5, 0.5);
    assert!(
        s.run_until(150, |s| s.snap.me.pos.dist(Vec3::new(10.5, 64.0, 0.5))
            <= 3.0)
    );
    s.snap.companion = None;
    for _ in 0..10 {
        s.step();
    }
    assert!(s.goal_text("goal_blocked").contains("not visible"));
    assert!(!s.has_event("goal_done"), "follow is open-ended");
}

#[test]
fn come_here_reaches_companion_or_fails_when_unseen() {
    let mut s = Sim::new();
    s.companion_at(9.5, -6.5);
    s.goal("come_here", false);
    assert!(s.run_until(200, |s| s.has_event("goal_done")));
    let c = s.snap.companion.as_ref().unwrap().pos;
    assert!(s.snap.me.pos.dist(c) <= 2.6);

    let mut s = Sim::new();
    s.goal("come_here", false);
    assert!(s.run_until(200, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("not visible"));

    let mut s = Sim::new();
    s.companion_at(9.5, 0.5);
    s.nav.state().unreachable_entities.push(COMPANION_ID);
    s.goal("come_here", false);
    assert!(s.run_until(200, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("cannot reach"));
}

#[test]
fn stay_walks_back_when_displaced() {
    let mut s = Sim::new();
    s.goal("stay", true);
    s.step();
    s.snap.me.pos = Vec3::new(4.5, 64.0, 0.5);
    assert!(s.run_until(60, |s| s.snap.me.pos.dist(Vec3::new(0.5, 64.0, 0.5)) <= 1.5));
    assert_eq!(s.exec.owner(Channel::Legs), Some("GOAL"));
}
