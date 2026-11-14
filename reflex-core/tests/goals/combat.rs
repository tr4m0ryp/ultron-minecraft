use crate::sim::{Sim, COMPANION_ID};
use reflex_core::knobs::Scope;
use reflex_core::snapshot::EntityKind;

#[test]
fn attack_by_id_chases_and_kills() {
    let mut s = Sim::new();
    s.mob(11, EntityKind::Passive, 12.5, 4.5);
    s.goal("attack #11", true);
    assert!(s.run_until(400, |s| s.has_event("goal_done")));
    assert!(s.attacked.iter().filter(|&&id| id == 11).count() >= 3);
    assert!(s.goal_text("goal_done").contains("gone after"));
}

#[test]
fn attack_target_killed_by_someone_else_is_done_not_lost() {
    // Live bug: the melee reflex killed the zombie while it held the body; the goal had
    // landed no hit and reported "lost sight of the zombie".
    let mut s = Sim::new();
    s.mob(13, EntityKind::Zombie, 5.5, 0.5);
    s.goal("attack zombie", true);
    s.step();
    s.snap.entities.retain(|e| e.id != 13);
    assert!(s.run_until(100, |s| s.has_event("goal_done")));
    assert!(s.goal_text("goal_done").contains("zombie is dead"));

    let mut s = Sim::new();
    s.mob(14, EntityKind::Zombie, 20.5, 0.5);
    s.goal("attack zombie", true);
    s.step();
    s.snap.entities.retain(|e| e.id != 14);
    assert!(s.run_until(100, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("lost sight"));
}

#[test]
fn attack_respects_the_mask_and_reports_missing_targets() {
    let mut s = Sim::new();
    s.mob(12, EntityKind::Villager, 4.5, 0.5);
    s.goal("attack villager", true);
    assert!(s.run_until(10, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("attack mask"));
    assert!(s.attacked.is_empty());

    let mut s = Sim::new();
    s.mob(12, EntityKind::Villager, 4.5, 0.5);
    s.exec
        .knobs
        .inhibit("D11", Scope::Everywhere, 1.0, Some(400), 1000);
    s.goal("attack villager", true);
    assert!(
        s.run_until(300, |s| s.has_event("goal_done")),
        "brain lifted the mask"
    );

    let mut s = Sim::new();
    s.goal("attack skeleton", true);
    assert!(s.run_until(10, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("no skeleton in view"));
}

#[test]
fn guard_fights_threats_near_the_companion_but_never_the_companion() {
    let mut s = Sim::new();
    s.companion_at(2.5, 0.5);
    s.mob(COMPANION_ID, EntityKind::Player, 2.5, 0.5);
    s.goal("guard", false);
    for _ in 0..20 {
        s.step();
    }
    s.mob(21, EntityKind::Witch, 7.5, 0.5);
    assert!(
        s.run_until(300, |s| s.snap.entity(21).is_none()),
        "witch dealt with"
    );
    assert!(s.goal_text("goal_progress").contains("engaging witch"));
    assert!(!s.attacked.contains(&COMPANION_ID));
    // The companion standing in the strike line is never a target, even with D11 lifted.
    s.exec
        .knobs
        .inhibit("D11", Scope::Everywhere, 1.0, Some(400), s.snap.tick);
    s.snap.damage.push(reflex_core::snapshot::DamageEvent {
        amount: 1.0,
        source_pos: None,
        attacker: Some(COMPANION_ID),
    });
    for _ in 0..100 {
        s.step();
    }
    assert!(!s.attacked.contains(&COMPANION_ID));
    assert!(!s.has_event("goal_done"), "guard is open-ended");
}
