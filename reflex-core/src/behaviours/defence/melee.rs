use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::behaviours::common::{heading_to, open_retreat};
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

/// K1 melee micro: draw the sword, close in, swing only when the cooldown is ready,
/// strafe inside the reach boundary.
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
    let legs = if d < STRAFE_INSIDE {
        let side = if (snap.tick / 20).is_multiple_of(2) {
            1.0
        } else {
            -1.0
        };
        LegsCmd::toward(
            Vec3::new(-toward.z * side, 0.0, toward.x * side),
            Speed::Walk,
        )
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
