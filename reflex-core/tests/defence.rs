mod common;

use common::*;
use reflex_core::behaviours::defence::*;
use reflex_core::channels::{Commands, GazeCmd, MainCmd, OffCmd, Speed};
use reflex_core::knobs::GainName;
use reflex_core::math::yaw_pitch_to;
use reflex_core::snapshot::{EntityKind, ItemKind, ItemStack};
use reflex_core::{Priority, Vec3};

#[test]
fn d1_creeper_backs_away_facing_it_and_flees_when_lit() {
    let mut s = flat();
    s.entities.push(mob(1, EntityKind::Creeper, 0.5, 20.0));
    assert!(
        eval(&mut CreeperStance, &s, &knobs()).0.is_none(),
        "beyond 16"
    );
    let mut c = mob(1, EntityKind::Creeper, 0.5, 6.0);
    c.vel = Vec3::new(0.0, 0.0, -0.2);
    s.entities = vec![c.clone()];
    let bid = eval(&mut CreeperStance, &s, &knobs()).0.unwrap();
    assert_eq!(bid.priority, Priority::P2);
    assert!(matches!(bid.commands.gaze, Some(GazeCmd::LookAt(_))));
    assert!(
        bid.commands.legs.unwrap().heading.z < -0.9,
        "moves away (toward -z)"
    );
    assert!(bid.degraded.is_some());

    c.pos = Vec3::new(0.5, 64.0, 2.5);
    c.state.fuse_lit = true;
    s.entities = vec![c];
    let bid = eval(&mut CreeperStance, &s, &knobs()).0.unwrap();
    assert_eq!(bid.priority, Priority::P0, "lit within 3");
    assert_eq!(bid.commands.legs.unwrap().speed, Speed::Sprint);
    s.me.offhand = Some(ItemStack::new(ItemKind::Shield, 1));
    let bid = eval(&mut CreeperStance, &s, &knobs()).0.unwrap();
    assert_eq!(bid.commands.off, Some(OffCmd::RaiseShield));
}

#[test]
fn d2_melee_fights_or_retreats_by_aggression_and_hp_floor() {
    let mut s = flat();
    assert!(eval(&mut MeleeDefence, &s, &knobs()).0.is_none());
    s.entities.push(mob(3, EntityKind::Zombie, 2.5, 0.5));
    let bid = eval(&mut MeleeDefence, &s, &knobs()).0.unwrap();
    assert_eq!(bid.note, "melee_fight");
    assert_eq!(
        bid.commands.main,
        Some(MainCmd::Attack(3)),
        "cooldown ready, in reach"
    );
    s.me.attack_ready = 0.5;
    assert_eq!(
        eval(&mut MeleeDefence, &s, &knobs())
            .0
            .unwrap()
            .commands
            .main,
        None,
        "waits for cooldown"
    );

    let mut k = knobs();
    k.set_gain(GainName::Aggression, 0.2);
    assert_eq!(
        eval(&mut MeleeDefence, &s, &k).0.unwrap().note,
        "melee_retreat"
    );
    s.me.health = 6.0;
    assert_eq!(
        eval(&mut MeleeDefence, &s, &knobs()).0.unwrap().note,
        "melee_retreat",
        "hp_floor forces flight"
    );

    let mut s = flat();
    s.entities.push(mob(4, EntityKind::Spider, 3.0, 0.5));
    assert!(
        eval(&mut MeleeDefence, &s, &knobs()).0.is_none(),
        "spiders are neutral by day"
    );
    s.is_night = true;
    assert!(eval(&mut MeleeDefence, &s, &knobs()).0.is_some());
}

#[test]
fn d5_gaze_filter_never_looks_at_enderman_head() {
    let mut s = flat();
    let e = mob(9, EntityKind::Enderman, 0.5, 10.0);
    let (yaw, pitch) = yaw_pitch_to(s.me.eye(), e.head());
    s.entities.push(e);
    let mut plan = Commands::default().gaze(GazeCmd::Angles { yaw, pitch });
    assert!(veto(&mut EndermanGazeFilter, &s, &knobs(), &mut plan).is_some());
    let Some(GazeCmd::Angles { pitch: p2, .. }) = plan.gaze else {
        panic!()
    };
    assert!((p2 - pitch).abs() > 10.0);

    let mut plan = Commands::default().gaze(GazeCmd::Angles {
        yaw: yaw + 90.0,
        pitch,
    });
    assert!(
        veto(&mut EndermanGazeFilter, &s, &knobs(), &mut plan).is_none(),
        "looking elsewhere"
    );
    s.me.wearing_pumpkin = true;
    let mut plan = Commands::default().gaze(GazeCmd::Angles { yaw, pitch });
    assert!(
        veto(&mut EndermanGazeFilter, &s, &knobs(), &mut plan).is_none(),
        "pumpkin"
    );
}

#[test]
fn d11_attack_mask_protects_neutrals_and_companion() {
    let mut s = flat();
    s.entities.push(mob(1, EntityKind::Villager, 2.0, 0.5));
    s.entities.push(mob(2, EntityKind::Zombie, 2.0, 1.5));
    let mut plan = Commands::default().main(MainCmd::Attack(1));
    assert!(veto(&mut AttackMask, &s, &knobs(), &mut plan).is_some());
    assert!(plan.main.is_none());
    let mut plan = Commands::default().main(MainCmd::Attack(2));
    assert!(veto(&mut AttackMask, &s, &knobs(), &mut plan).is_none());
}

#[test]
fn r14_projectile_shield_or_strafe() {
    let mut s = flat();
    let mut arrow = mob(5, EntityKind::Arrow, 0.5, 9.5);
    arrow.pos.y = 65.0;
    arrow.vel = Vec3::new(0.0, 0.0, -1.5);
    s.entities.push(arrow.clone());
    let bid = eval(&mut ProjectileEvade, &s, &knobs()).0.unwrap();
    assert_eq!(bid.note, "strafe_projectile", "no shield");
    assert!(
        bid.commands.legs.unwrap().heading.z.abs() < 0.1,
        "perpendicular to flight"
    );
    s.me.offhand = Some(ItemStack::new(ItemKind::Shield, 1));
    let bid = eval(&mut ProjectileEvade, &s, &knobs()).0.unwrap();
    assert_eq!(
        bid.commands.off,
        Some(OffCmd::RaiseShield),
        "6 ticks out: time to raise"
    );
    arrow.pos.z = 5.0;
    s.entities = vec![arrow.clone()];
    assert_eq!(
        eval(&mut ProjectileEvade, &s, &knobs()).0.unwrap().note,
        "strafe_projectile",
        "too late to raise"
    );
    arrow.pos.x = 5.0;
    s.entities = vec![arrow];
    assert!(
        eval(&mut ProjectileEvade, &s, &knobs()).0.is_none(),
        "misses by 4.5 blocks"
    );
}

#[test]
fn d3_ranged_cover_or_close_in() {
    let mut s = flat();
    let mut sk = mob(6, EntityKind::Skeleton, 0.5, 10.0);
    sk.state.drawing_bow = true;
    s.entities.push(sk);
    s.me.offhand = Some(ItemStack::new(ItemKind::Shield, 1));
    let bid = eval(&mut RangedCover, &s, &knobs()).0.unwrap();
    assert_eq!(
        (bid.note, bid.commands.off),
        ("ranged_cover", Some(OffCmd::RaiseShield))
    );
    let mut k = knobs();
    k.set_gain(GainName::Aggression, 0.8);
    assert_eq!(
        eval(&mut RangedCover, &s, &k).0.unwrap().note,
        "ranged_close_in"
    );
    s.entities.clear();
    assert!(eval(&mut RangedCover, &s, &knobs()).0.is_none());
}

/// Live, with several zombies, O2 turned toward the next approaching one, kept the gaze by
/// hysteresis, and D2's fight (which needed the gaze) lost every channel: 5 swings in 225 s.
#[test]
fn d2_keeps_swinging_while_o2_holds_the_gaze() {
    let mut ex = reflex_core::Executor::new(BOT);
    let mut s = flat();
    give(&mut s, 0, ItemKind::Sword, 1);
    let mut far = mob(2, EntityKind::Zombie, 0.5, 9.5);
    far.vel = Vec3::new(0.0, 0.0, -1.0);
    s.entities.push(far);
    ex.tick(&s);
    assert_eq!(ex.owner(reflex_core::Channel::Gaze), Some("O2"));
    s.tick += 1;
    s.entities.push(mob(3, EntityKind::Zombie, 2.5, 0.5));
    let out = ex.tick(&s);
    assert_eq!(out.main, Some(MainCmd::Attack(3)));
}
