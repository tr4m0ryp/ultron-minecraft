mod common;

use common::*;
use reflex_core::behaviours::homeostasis::{CompanionLeash, Health, Hunger, GOAL_GRACE};
use reflex_core::behaviours::locomotion::Unstuck;
use reflex_core::behaviours::orient::{LookAtSpeaker, Looming};
use reflex_core::channels::{Channel, InvCmd, MainCmd};
use reflex_core::knobs::GainName;
use reflex_core::predict::Efference;
use reflex_core::snapshot::{ChatEvent, Companion, EntityKind, ItemKind, ItemStack};
use reflex_core::{EventClass, Executor, Priority, Vec3};

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

fn companion_at(x: f64) -> Companion {
    Companion {
        name: "human".into(),
        entity_id: Some(99),
        pos: Vec3::new(x, 64.0, 0.5),
        vel: Vec3::ZERO,
        health: Some(20.0),
    }
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

#[test]
fn o1_looks_at_speaker_for_40_ticks() {
    let mut s = flat();
    let mut b = LookAtSpeaker::default();
    s.chat.push(ChatEvent {
        sender: "far".into(),
        text: "hello".into(),
        sender_pos: Some(Vec3::new(50.0, 64.0, 0.0)),
    });
    assert!(
        eval(&mut b, &s, &knobs()).0.is_none(),
        "far and not naming the bot"
    );
    s.chat = vec![ChatEvent {
        sender: "human".into(),
        text: "ultron, come here".into(),
        sender_pos: Some(Vec3::new(5.0, 64.0, 0.5)),
    }];
    let (bid, ev) = eval(&mut b, &s, &knobs());
    assert!(bid.unwrap().commands.gaze.is_some());
    assert!(ev
        .iter()
        .any(|e| e.kind == "chat" && e.class == EventClass::Wake));
    s.chat.clear();
    s.tick += 39;
    assert!(eval(&mut b, &s, &knobs()).0.is_some(), "still holding");
    s.tick += 1;
    assert!(eval(&mut b, &s, &knobs()).0.is_none(), "hold expired");
}

#[test]
fn o1_wakes_on_the_companion_at_any_distance() {
    let mut s = flat();
    let mut b = LookAtSpeaker::default();
    s.companion = Some(companion_at(40.0));
    s.chat.push(ChatEvent {
        sender: "human".into(),
        text: "can you come over here?".into(),
        sender_pos: Some(Vec3::new(40.0, 64.0, 0.5)),
    });
    let (_, ev) = eval(&mut b, &s, &knobs());
    assert!(
        ev.iter()
            .any(|e| e.kind == "chat" && e.class == EventClass::Wake),
        "companion line 40 blocks off, no name"
    );
}

#[test]
fn o2_looming_orients_and_habituates_to_harmless() {
    let mut s = flat();
    let mut z = mob(1, EntityKind::Zombie, 8.0, 0.5);
    s.entities.push(z.clone());
    assert!(
        eval(&mut Looming::default(), &s, &knobs()).0.is_none(),
        "not approaching"
    );
    z.vel = Vec3::new(-0.3, 0.0, 0.0);
    s.entities = vec![z];
    assert_eq!(
        eval(&mut Looming::default(), &s, &knobs())
            .0
            .unwrap()
            .priority,
        Priority::P2
    );

    let mut cow = mob(2, EntityKind::Passive, 8.0, 0.5);
    cow.vel = Vec3::new(-0.3, 0.0, 0.0);
    s.entities = vec![cow];
    let mut b = Looming::default();
    assert_eq!(eval(&mut b, &s, &knobs()).0.unwrap().priority, Priority::P4);
    s.tick += 10;
    assert!(
        eval(&mut b, &s, &knobs()).0.is_none(),
        "classified harmless"
    );
    s.tick += 100;
    assert!(
        eval(&mut b, &s, &knobs()).0.is_none(),
        "inhibition of return"
    );
    s.tick += 200;
    assert!(eval(&mut b, &s, &knobs()).0.is_some(), "recovered");
}

#[test]
fn c6_unstuck_escalates_then_wakes_over_a_tick_sequence() {
    let mut s = flat();
    let mut ex = Executor::with_behaviours(
        BOT,
        vec![
            Box::new(Unstuck::default()),
            Box::new(reflex_core::behaviours::GoalExecutor::default()),
        ],
    );
    ex.knobs
        .set_goal("walk east", 0.8, Some(Vec3::new(20.0, 64.0, 0.5)), false);
    let mut wake = Vec::new();
    for t in 0..330 {
        s.tick = 1000 + t;
        ex.tick(&s);
        if t == 5 {
            assert_eq!(ex.owner(Channel::Legs), Some("GOAL"));
        }
        if t == 30 {
            assert_eq!(
                ex.owner(Channel::Legs),
                Some("C6"),
                "unstuck took the legs after 20 stuck ticks"
            );
        }
        wake.extend(ex.take_wake(s.tick));
    }
    assert!(wake.iter().any(|e| e.kind == "unstuck_failed"));
    assert!(wake.iter().any(|e| e.kind == "goal_interrupted"));

    let mut b = Unstuck::default();
    let eff = Efference {
        legs: Some(reflex_core::channels::LegsCmd::toward(
            Vec3::new(1.0, 0.0, 0.0),
            reflex_core::channels::Speed::Walk,
        )),
        from: s.me.pos - Vec3::new(0.2, 0.0, 0.0),
    };
    for _ in 0..30 {
        assert!(
            eval_eff(&mut b, &s, &knobs(), eff).0.is_none(),
            "moving, not stuck"
        );
    }
}

#[test]
fn r11_totem_to_offhand_when_death_is_plausible() {
    use reflex_core::behaviours::defence::Totem;
    let mut s = flat();
    s.me.inventory
        .push((12, ItemStack::new(ItemKind::Totem, 1)));
    assert!(eval(&mut Totem, &s, &knobs()).0.is_none());
    s.me.health = 5.0;
    let bid = eval(&mut Totem, &s, &knobs()).0.unwrap();
    assert_eq!(
        (bid.priority, bid.commands.inv),
        (Priority::P0, Some(InvCmd::ToOffhand(12)))
    );
    s.me.offhand = Some(ItemStack::new(ItemKind::Totem, 1));
    assert!(
        eval(&mut Totem, &s, &knobs()).0.is_none(),
        "already equipped"
    );
}
