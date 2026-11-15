//! Orienting (O1, O2), the unstuck pattern (C6) and the totem reflex (R11), split from
//! drives.rs to keep each test file under 300 lines.
mod common;

use common::*;
use reflex_core::behaviours::locomotion::Unstuck;
use reflex_core::behaviours::orient::{LookAtSpeaker, Looming};
use reflex_core::channels::{Channel, InvCmd, MainCmd};
use reflex_core::predict::Efference;
use reflex_core::snapshot::{ChatEvent, EntityKind, ItemKind, ItemStack};
use reflex_core::{BlockPos, EventClass, Executor, Priority, Vec3};

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

/// Digs `C6` commands over a long stuck spell, with `kept` marking the cells ahead.
fn unstuck_digs(kept: bool) -> usize {
    let mut s = flat();
    let mut ex = Executor::with_behaviours(
        BOT,
        vec![
            Box::new(Unstuck::default()),
            Box::new(reflex_core::behaviours::GoalExecutor::default()),
        ],
    );
    if kept {
        // The player's own wall: every cell in front of the body, at both heights.
        ex.knobs.kept = (0..4)
            .flat_map(|dx| (0..3).map(move |dy| BlockPos::new(dx, 63 + dy, 0)))
            .collect();
    }
    ex.knobs
        .set_goal("walk east", 0.8, Some(Vec3::new(20.0, 64.0, 0.5)), false);
    let mut digs = 0;
    for t in 0..330 {
        s.tick = 1000 + t;
        let out = ex.tick(&s);
        digs += usize::from(matches!(out.main, Some(MainCmd::Dig(_))));
    }
    digs
}

#[test]
fn c6_unstuck_never_digs_the_players_own_blocks() {
    assert!(unstuck_digs(false) > 0, "control: the last stage digs");
    assert_eq!(unstuck_digs(true), 0, "a kept block is waited at, not dug");
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
