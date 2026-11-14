use crate::sim::Sim;
use reflex_core::goals::ToolNeed;
use reflex_core::snapshot::ItemKind;
use reflex_core::BlockPos;

#[test]
fn a_new_goal_replaces_the_old_one_which_resumes_with_its_progress() {
    let mut s = Sim::new();
    for x in [6, 7, 8] {
        s.block(BlockPos::new(x, 64, 6), "iron_ore");
    }
    s.hotbar(2, ItemKind::Pickaxe, 1);
    s.world
        .state()
        .tools
        .insert("iron_ore".into(), ToolNeed::Have(2));
    let mine = s.goal("mine iron_ore 3", true);
    assert!(
        s.run_until(600, |s| s.world.state().blocks.len() == 2),
        "one mined"
    );
    let cancels = s.nav.state().cancels;
    s.goal("go_to -6 64 -6", true);
    s.step();
    assert!(s.nav.state().cancels > cancels, "old request cancelled");
    assert!(s.has_event("goal_suspended"));
    assert!(s.run_until(200, |s| s.has_event("goal_done")));
    // Finishing the walk settles it; the queued mine goal comes back and resumes.
    assert_eq!(s.exec.knobs.goal.as_ref().map(|g| g.id), Some(mine));
    assert!(s.run_until(1500, |s| s.events("goal_done").len() == 2));
    assert!(s.has_event("goal_resumed"));
    assert!(
        s.goal_text("goal_done").contains("mined 3 iron_ore"),
        "progress kept: {}",
        s.goal_text("goal_done")
    );
    assert!(s.world.state().blocks.is_empty());
}

#[test]
fn ending_a_goal_stops_the_body_and_bad_specs_fail_once() {
    let mut s = Sim::new();
    s.goal("go_to 20 64 20", true);
    for _ in 0..10 {
        s.step();
    }
    s.exec.knobs.end_goal();
    let out = s.step();
    assert_eq!((out.forward, out.strafe), (0, 0));
    assert!(s.nav.state().current.is_none());

    s.goal("juggle", true);
    for _ in 0..20 {
        s.step();
    }
    assert_eq!(s.events("goal_failed").len(), 1);
    assert!(s.goal_text("goal_failed").contains("cannot parse"));
    assert!(s.exec.knobs.goal.is_none(), "settled");
}

#[test]
fn status_events_do_not_flood() {
    let mut s = Sim::new();
    s.companion_at(4.5, 0.5);
    s.goal("follow", false);
    s.snap.companion = None;
    for _ in 0..1000 {
        s.step();
    }
    let blocked = s.events("goal_blocked").len();
    assert_eq!(blocked, 1, "one report for a persistent reason");
}
