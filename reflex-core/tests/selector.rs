mod common;

use std::cell::Cell;
use std::rc::Rc;

use common::*;
use reflex_core::behaviour::{Behaviour, Bid, Ctx, Priority};
use reflex_core::channels::{Channel, Commands, LegsCmd, MainCmd, Speed};
use reflex_core::selector::salience;
use reflex_core::snapshot::{ChatEvent, EntityKind};
use reflex_core::{Executor, Vec3};

/// Test behaviour with externally controlled urgency and a fixed command set.
struct Probe {
    id: &'static str,
    p: Priority,
    u: Rc<Cell<f32>>,
    cmds: Commands,
    lock: u32,
}

impl Behaviour for Probe {
    fn id(&self) -> &'static str {
        self.id
    }
    fn evaluate(&mut self, _ctx: &Ctx) -> Option<Bid> {
        let u = self.u.get();
        (u > 0.0).then(|| Bid::new(self.p, u, "probe", self.cmds.clone()).lock(self.lock))
    }
}

fn probe(id: &'static str, p: Priority, u: f32, cmds: Commands) -> (Box<dyn Behaviour>, Rc<Cell<f32>>) {
    let cell = Rc::new(Cell::new(u));
    (Box::new(Probe { id, p, u: cell.clone(), cmds, lock: 0 }), cell)
}

#[test]
fn walks_backward_while_gazing_at_creeper() {
    let mut s = flat();
    let mut c = mob(1, EntityKind::Creeper, 0.5, 6.5);
    c.vel = Vec3::new(0.0, 0.0, -0.2);
    s.entities.push(c);
    let mut ex = Executor::new(BOT);
    let out = ex.tick(&s);
    assert_eq!(ex.owner(Channel::Legs), Some("D1"));
    assert_eq!(ex.owner(Channel::Gaze), Some("D1"));
    assert_eq!(out.forward, -1, "backpedal");
    assert_eq!(out.strafe, 0);
    assert!(out.yaw.abs() < 5.0, "faces +z, toward the creeper: yaw {}", out.yaw);
    assert!(!out.sprint, "no sprint while walking backward");
}

#[test]
fn legs_and_gaze_split_between_goal_and_speaker() {
    let mut s = flat();
    let mut ex = Executor::new(BOT);
    ex.knobs.set_goal("go east", 0.6, Some(Vec3::new(6.5, 64.0, 0.5)), false);
    s.chat.push(ChatEvent { sender: "human".into(), text: "hi".into(), sender_pos: Some(Vec3::new(0.5, 64.0, 8.0)) });
    let out = ex.tick(&s);
    assert_eq!(ex.owner(Channel::Legs), Some("GOAL"));
    assert_eq!(ex.owner(Channel::Gaze), Some("O1"));
    assert_eq!((out.forward, out.strafe), (0, -1), "looking +z while walking +x is a left strafe");
}

#[test]
fn salience_classes_never_overlap_and_gain_scales_within_class() {
    let p1_weak = salience(Priority::P1, 0.1, 0.1, 0.0).value;
    let p2_strong = salience(Priority::P2, 5.0, 1.0, 0.0).value;
    assert!(p1_weak > p2_strong);
    assert!(salience(Priority::P3, 0.8, 0.5, 0.0).value > salience(Priority::P3, 0.4, 0.5, 0.0).value);
    let inh = salience(Priority::P1, 1.0, 1.0, 1.0);
    assert_eq!(inh.drive, 0.0);
    let floor = salience(Priority::P0, 1.0, 1.0, 1.0);
    assert!(floor.overridden && floor.drive > 0.9);
}

#[test]
fn hysteresis_keeps_holder_until_margin_exceeded() {
    let legs = Commands::default().legs(LegsCmd::toward(Vec3::new(1.0, 0.0, 0.0), Speed::Walk));
    let (a, ua) = probe("A", Priority::P4, 0.50, legs.clone());
    let (b, ub) = probe("B", Priority::P4, 0.10, legs);
    let mut ex = Executor::with_behaviours(BOT, vec![a, b]);
    let mut s = flat();
    ex.tick(&s);
    assert_eq!(ex.owner(Channel::Legs), Some("A"));
    ub.set(0.58);
    s.tick += 1;
    ex.tick(&s);
    assert_eq!(ex.owner(Channel::Legs), Some("A"), "B ahead by 0.08 < 0.1");
    ub.set(0.65);
    s.tick += 1;
    ex.tick(&s);
    assert_eq!(ex.owner(Channel::Legs), Some("B"), "B ahead by 0.15");
    ua.set(0.0);
    ub.set(0.0);
    s.tick += 1;
    ex.tick(&s);
    assert_eq!(ex.owner(Channel::Legs), None);
}

#[test]
fn per_behaviour_gain_decides_within_a_class() {
    let legs = Commands::default().legs(LegsCmd::toward(Vec3::new(1.0, 0.0, 0.0), Speed::Walk));
    let (a, _) = probe("A", Priority::P4, 0.5, legs.clone());
    let (b, _) = probe("B", Priority::P4, 0.4, legs);
    let mut ex = Executor::with_behaviours(BOT, vec![a, b]);
    ex.knobs.gains.per_behaviour.insert("B".into(), 2.0);
    ex.tick(&flat());
    assert_eq!(ex.owner(Channel::Legs), Some("B"));
}

#[test]
fn commitment_lock_holds_against_non_p0_and_breaks_for_p0() {
    let eat = Commands::default().main(MainCmd::Use);
    let cell = Rc::new(Cell::new(0.5));
    let eater = Box::new(Probe { id: "EAT", p: Priority::P3, u: cell.clone(), cmds: eat.clone(), lock: 32 });
    let (rival, ur) = probe("RIVAL", Priority::P2, 0.0, Commands::default().main(MainCmd::Attack(1)));
    let (lethal, ul) = probe("LETHAL", Priority::P0, 0.0, Commands::default().main(MainCmd::SelectSlot(3)));
    let mut ex = Executor::with_behaviours(BOT, vec![eater, rival, lethal]);
    let mut s = flat();
    ex.tick(&s);
    assert_eq!(ex.owner(Channel::Main), Some("EAT"));
    ur.set(1.0);
    for _ in 0..10 {
        s.tick += 1;
        ex.tick(&s);
        assert_eq!(ex.owner(Channel::Main), Some("EAT"), "P2 cannot break the eating lock");
    }
    ul.set(1.0);
    s.tick += 1;
    ex.tick(&s);
    assert_eq!(ex.owner(Channel::Main), Some("LETHAL"));
}

#[test]
fn degraded_variant_takes_legs_when_gaze_is_owned_higher() {
    let mut s = flat();
    let mut c = mob(1, EntityKind::Creeper, 0.5, 6.5);
    c.vel = Vec3::new(0.0, 0.0, -0.2);
    s.entities.push(c);
    let (look, _) = probe("LOOK", Priority::P1, 1.0, Commands::default().look_at(Vec3::new(10.0, 64.0, 0.0)));
    let mut cat = reflex_core::behaviours::default_catalog();
    cat.push(look);
    let mut ex = Executor::with_behaviours(BOT, cat);
    ex.tick(&s);
    assert_eq!(ex.owner(Channel::Gaze), Some("LOOK"));
    assert_eq!(ex.owner(Channel::Legs), Some("D1"), "D1 degraded to legs only");
}

#[test]
fn p0_lava_beats_p1_contact_on_legs() {
    let mut s = flat();
    s.blocks.set(reflex_core::BlockPos::new(0, 64, 0), reflex_core::snapshot::BlockKind::Lava);
    s.blocks.set(reflex_core::BlockPos::new(0, 65, 0), reflex_core::snapshot::BlockKind::Fire);
    let mut ex = Executor::new(BOT);
    ex.tick(&s);
    assert_eq!(ex.owner(Channel::Legs), Some("R1"));
}
