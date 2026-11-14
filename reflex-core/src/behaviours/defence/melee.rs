use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::behaviours::common::{clearance, heading_to, knockback_room, open_retreat, Bound};
use crate::channels::{Commands, LegsCmd, MainCmd, Speed};
use crate::math::Vec3;
use crate::snapshot::{Entity, EntityKind, ItemKind, Snapshot};

const ENGAGE: f64 = 8.0;
const BABY_RETREAT: f64 = 10.0;
const REACH: f64 = 3.0;
const STRAFE_INSIDE: f64 = 2.5;
const SWARM: usize = 3;
const SWARM_RADIUS: f64 = 6.0;
/// Cornered, fight back once the foe is this close (zombie reach is about 2 blocks).
const CORNERED_REACH: f64 = 3.5;

/// A mob hit throws the player about 2.5 blocks (0.4 blocks/tick, air drag 0.91, roughly 10
/// airborne ticks); keep this much ground behind the back when in reach of a foe.
const KNOCKBACK_ROOM: f64 = 3.0;
/// Probe length for one footing step.
const FOOTING_STEP: f64 = 0.8;

fn rotate(v: Vec3, deg: f64) -> Vec3 {
    let (s, c) = deg.to_radians().sin_cos();
    Vec3::new(v.x * c - v.z * s, 0.0, v.x * s + v.z * c)
}

/// Combat footing: with a drop closer behind than a knockback carries, step around the foe
/// (sideways, or diagonally past it) to the spot with the most room behind, as a skilled
/// player circles so the edge ends up behind the mob instead. `None` when the footing is
/// fine or no step improves it.
pub fn footing(snap: &Snapshot, threat: Vec3) -> Option<LegsCmd> {
    let me = snap.me.pos;
    let blocks = &snap.blocks;
    let room = knockback_room(blocks, me, threat, KNOCKBACK_ROOM);
    if room >= KNOCKBACK_ROOM {
        return None;
    }
    let toward = heading_to(me, threat);
    let mut best: Option<(f64, Vec3)> = None;
    for deg in [90.0, -90.0, 45.0, -45.0, 135.0, -135.0] {
        let h = rotate(toward, deg);
        if clearance(blocks, me, h, FOOTING_STEP) != (FOOTING_STEP, Bound::Open) {
            continue;
        }
        let after = knockback_room(blocks, me + h * FOOTING_STEP, threat, KNOCKBACK_ROOM);
        if after > room + 0.25 && best.is_none_or(|(r, _)| after > r) {
            best = Some((after, h));
        }
    }
    best.map(|(_, h)| LegsCmd::toward(h, Speed::Walk))
}

/// K1 melee micro: draw the sword, close in, swing only when the cooldown is ready,
/// strafe inside the reach boundary, and keep the back away from drops.
pub fn melee_commands(snap: &Snapshot, target: &Entity) -> Commands {
    let me = &snap.me;
    let d = target.pos.dist(me.pos);
    let face = Commands::default().look_at(target.head());
    if let Some(slot) = me.hotbar_find(|k| k == ItemKind::Sword) {
        if me.selected_slot != slot {
            return face.main(MainCmd::SelectSlot(slot));
        }
    }
    let toward = heading_to(me.pos, target.pos);
    if d > REACH {
        return face.legs(LegsCmd::toward(toward, Speed::Sprint));
    }
    let legs = if let Some(step) = footing(snap, target.pos) {
        step
    } else if d < STRAFE_INSIDE {
        let side = if (snap.tick / 20).is_multiple_of(2) {
            1.0
        } else {
            -1.0
        };
        let strafe = |k: f64| Vec3::new(-toward.z * k, 0.0, toward.x * k);
        let open = |h: Vec3| clearance(&snap.blocks, me.pos, h, 1.0).1 != Bound::Drop;
        match [side, -side].into_iter().map(strafe).find(|&h| open(h)) {
            Some(h) => LegsCmd::toward(h, Speed::Walk),
            None => LegsCmd::stop(),
        }
    } else {
        LegsCmd::stop()
    };
    let face = face.legs(legs);
    if me.attack_ready >= 0.95 {
        face.main(MainCmd::Attack(target.id))
    } else {
        face
    }
}

/// D2: zombie, husk, drowned, and spiders at night. Fight with K1 when aggression >= 0.5
/// and hp above hp_floor; otherwise, or when three or more melee mobs are within 6, retreat
/// toward the companion or away. Baby zombies trigger the retreat from 10 blocks. Cornered
/// (no safe retreat heading) with the foe within reach: fight back with K1 instead of
/// pressing against the edge.
#[derive(Default)]
pub struct MeleeDefence;

impl Behaviour for MeleeDefence {
    fn id(&self) -> &'static str {
        "D2"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let snap = ctx.snap;
        let me = &snap.me;
        let relevant = |e: &Entity| {
            e.kind.is_melee()
                && !(e.kind == EntityKind::Drowned && e.state.holding_trident)
                && !(matches!(e.kind, EntityKind::Spider | EntityKind::CaveSpider)
                    && !snap.is_night)
        };
        let radius = |e: &Entity| if e.state.baby { BABY_RETREAT } else { ENGAGE };
        let foes: Vec<&Entity> = snap.nearby(BABY_RETREAT, |e| {
            relevant(e) && e.pos.dist(me.pos) <= radius(e)
        });
        let target = *foes.first()?;
        let swarm = snap.nearby(SWARM_RADIUS, |e| relevant(e)).len() >= SWARM;
        let g = &ctx.knobs.gains;
        let fight = g.aggression >= 0.5 && me.health > g.hp_floor && !swarm && !target.state.baby;
        let d = target.pos.dist(me.pos);
        let urgency = (1.0 - d / BABY_RETREAT).clamp(0.2, 1.0) as f32;
        if fight {
            return Some(Bid::new(
                Priority::P2,
                urgency,
                "melee_fight",
                melee_commands(snap, target),
            ));
        }
        let heading = snap
            .companion
            .as_ref()
            .filter(|c| c.pos.dist(target.pos) > d)
            .map(|c| heading_to(me.pos, c.pos))
            .or_else(|| open_retreat(snap, target.pos));
        let Some(heading) = heading else {
            if d <= CORNERED_REACH {
                let cmds = melee_commands(snap, target);
                return Some(Bid::new(Priority::P2, 1.0, "melee_cornered", cmds));
            }
            return None;
        };
        let cmds = Commands::default().legs(LegsCmd::toward(heading, Speed::Sprint));
        Some(Bid::new(Priority::P2, urgency, "melee_retreat", cmds))
    }
}
