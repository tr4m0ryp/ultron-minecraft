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

/// One `Attack` step against a zombie at `dist`, with the sword in slot 0 selected.
fn attack_step(
    dist: f64,
    prep: impl FnOnce(&mut reflex_core::snapshot::Snapshot),
) -> reflex_core::goals::Step {
    use reflex_core::behaviour::Ctx;
    use reflex_core::goals::fakes::{FakeNav, FakeWorld};
    use reflex_core::goals::{AttackTarget, GoalCx, GoalTask, Reporter};
    use reflex_core::knobs::Knobs;
    use reflex_core::predict::{Efference, Prediction};
    let mut s = crate::sim::Sim::new().snap;
    s.me.hotbar[0] = Some(reflex_core::snapshot::ItemStack::new(
        reflex_core::snapshot::ItemKind::Sword,
        1,
    ));
    s.me.selected_slot = 0;
    s.entities.push(reflex_core::snapshot::Entity::new(
        5,
        EntityKind::Zombie,
        reflex_core::Vec3::new(0.5 + dist, 64.0, 0.5),
    ));
    prep(&mut s);
    let pred = Prediction::compute(&s, Efference::default());
    let knobs = Knobs::new();
    let ctx = Ctx::new(&s, &pred, &knobs, "Ultron");
    let (mut nav, mut world) = (FakeNav::default(), FakeWorld::default());
    let mut rep = Reporter::new(1, "attack");
    let mut cx = GoalCx {
        ctx: &ctx,
        nav: &mut nav,
        world: &mut world,
        rep: &mut rep,
        ledger: &reflex_core::goals::Ledger::new(),
        preempted: false,
    };
    reflex_core::goals::combat::Attack::new(AttackTarget::Id(5)).step(&mut cx)
}

#[test]
fn attack_goal_swings_with_the_k1_micro_and_keeps_the_strike_when_gaze_is_lost() {
    use reflex_core::channels::MainCmd;
    use reflex_core::goals::Step;
    let Step::Bid(plan) = attack_step(2.0, |_| {}) else {
        panic!("attack bids")
    };
    assert_eq!(plan.commands.main, Some(MainCmd::Attack(5)));
    assert!(plan.commands.gaze.is_some());
    // O2 (P2) takes the gaze from a P5 goal whenever a mob closes; the fallback must still
    // strike or the goal never lands a blow.
    let degraded = plan.degraded.expect("fallback plan");
    assert_eq!(degraded.main, Some(MainCmd::Attack(5)));
    assert!(degraded.gaze.is_none());

    let Step::Bid(plan) = attack_step(2.0, |s| s.me.attack_ready = 0.5) else {
        panic!("attack bids")
    };
    assert_eq!(plan.commands.main, None, "no swing while the cooldown runs");
}

#[test]
fn attack_goal_draws_the_sword_before_closing_and_falls_back_to_an_axe() {
    use reflex_core::channels::MainCmd;
    use reflex_core::goals::Step;
    use reflex_core::snapshot::{ItemKind, ItemStack};
    let Step::Bid(plan) = attack_step(2.0, |s| s.me.selected_slot = 3) else {
        panic!("attack bids")
    };
    assert_eq!(plan.commands.main, Some(MainCmd::SelectSlot(0)));
    let Step::Bid(plan) = attack_step(2.0, |s| {
        s.me.hotbar[0] = None;
        s.me.hotbar[4] = Some(ItemStack::new(ItemKind::Axe, 1));
        s.me.selected_slot = 1;
    }) else {
        panic!("attack bids")
    };
    assert_eq!(plan.commands.main, Some(MainCmd::SelectSlot(4)));
}
