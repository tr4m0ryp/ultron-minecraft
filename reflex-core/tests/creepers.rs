//! D1 creeper duel: hit and retreat between fuse starts, plain retreat when it is unsafe.
mod common;

use common::*;
use reflex_core::behaviours::defence::CreeperStance;
use reflex_core::channels::{MainCmd, Speed};
use reflex_core::snapshot::{Entity, EntityKind, ItemKind};
use reflex_core::Priority;

fn creeper(z: f64, swell: Option<u8>) -> Entity {
    let mut c = mob(1, EntityKind::Creeper, 0.5, z);
    if let Some(s) = swell {
        c.state.fuse_lit = true;
        c.state.swell = s;
    }
    c
}

fn armed() -> reflex_core::snapshot::Snapshot {
    let mut s = flat();
    give(&mut s, 0, ItemKind::Sword, 1);
    s
}

#[test]
fn duel_meets_strikes_and_recoils() {
    let mut s = armed();
    s.entities.push(creeper(6.5, None));
    let bid = eval(&mut CreeperStance, &s, &knobs()).0.unwrap();
    assert_eq!(bid.note, "creeper_engage");
    assert!(bid.commands.legs.unwrap().heading.z > 0.9);

    s.entities = vec![creeper(3.7, None)];
    let bid = eval(&mut CreeperStance, &s, &knobs()).0.unwrap();
    assert_eq!(bid.note, "creeper_strike");
    assert_eq!(bid.commands.main, Some(MainCmd::Attack(1)));
    assert_eq!(
        bid.commands.legs.unwrap().speed,
        Speed::Sprint,
        "sprint hit"
    );

    s.entities = vec![creeper(4.5, Some(6))];
    s.me.attack_ready = 0.3;
    let bid = eval(&mut CreeperStance, &s, &knobs()).0.unwrap();
    assert_eq!(bid.note, "creeper_recoil");
    assert!(
        bid.commands.legs.unwrap().heading.z < -0.9,
        "back off past 7"
    );

    s.entities = vec![creeper(10.5, None)];
    assert_eq!(
        eval(&mut CreeperStance, &s, &knobs()).0.unwrap().note,
        "creeper_wait",
        "recharging out of fuse range"
    );
}

#[test]
fn late_fuse_low_health_or_a_second_creeper_falls_back_to_retreat() {
    let mut s = armed();
    s.entities.push(creeper(2.5, Some(20)));
    let bid = eval(&mut CreeperStance, &s, &knobs()).0.unwrap();
    assert_eq!((bid.note, bid.priority), ("creeper_flee", Priority::P0));

    s.entities = vec![creeper(3.7, None)];
    s.me.health = 5.0;
    assert_eq!(
        eval(&mut CreeperStance, &s, &knobs()).0.unwrap().note,
        "creeper_back_away"
    );

    s.me.health = 20.0;
    let mut other = mob(2, EntityKind::Creeper, 4.5, 2.5);
    other.vel = reflex_core::Vec3::new(-0.1, 0.0, 0.0);
    s.entities.push(other);
    assert_eq!(
        eval(&mut CreeperStance, &s, &knobs()).0.unwrap().note,
        "creeper_back_away"
    );
}
