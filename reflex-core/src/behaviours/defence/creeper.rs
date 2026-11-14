use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::behaviours::common::{heading_to, safe_retreat};
use crate::channels::{Commands, LegsCmd, MainCmd, OffCmd, Speed};
use crate::predict::tau;
use crate::snapshot::{Entity, EntityKind, ItemKind, Snapshot};

/// D1: creeper within 16. A creeper starts its 30-tick fuse when its target is within 3
/// blocks and keeps swelling until the target is more than 7 away, when the fuse burns back
/// down a tick at a time. With a sword, aggression >= 0.5, hp at or above hp_floor and no
/// second creeper close, duel it as a skilled player does: meet it at the edge of reach
/// (the hitbox is reachable from about 3.3, the fuse starts inside 3), sprint-hit it (the
/// sprint knockback throws it back), then back off past 7 so the fuse cools while the sword
/// recharges, and repeat; flee outright once the fuse is past `FLEE_SWELL`. Otherwise the
/// old stance: back away to keep 7 blocks while facing it; fuse lit within 7: raise a held
/// shield (explosions are blocked in Java), else sprint straight away (P2, P0 within 3).
#[derive(Default)]
pub struct CreeperStance;

const WATCH: f64 = 16.0;
const KEEP: f64 = 7.0;
const BLAST: f64 = 3.0;
/// Centre distance from which a swing reaches the creeper's hitbox.
const STRIKE: f64 = 3.3;
/// Past this fuse progress (12 ticks left, about 3.4 blocks of sprint) only fleeing is safe.
const FLEE_SWELL: u8 = 18;
/// Another creeper this close turns a duel into a retreat.
const CROWD: f64 = 8.0;

fn sword_slot(snap: &Snapshot) -> Option<u8> {
    snap.me.hotbar_find(|k| k == ItemKind::Sword)
}

impl Behaviour for CreeperStance {
    fn id(&self) -> &'static str {
        "D1"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let snap = ctx.snap;
        let me = &snap.me;
        let creepers = snap.nearby(WATCH, |e| e.kind == EntityKind::Creeper);
        let c = *creepers.first()?;
        let d = c.pos.dist(me.pos);
        let g = &ctx.knobs.gains;
        let crowded = creepers.iter().skip(1).any(|o| o.pos.dist(me.pos) < CROWD);
        let duel = g.aggression >= 0.5 && me.health >= g.hp_floor && !crowded;
        if duel && sword_slot(snap).is_some() {
            if let Some(bid) = duel_bid(snap, c, d) {
                return Some(bid);
            }
        }
        stance_bid(ctx, c, d)
    }
}

/// One tick of the hit-and-retreat duel; `None` hands over to the plain stance (flee).
fn duel_bid(snap: &Snapshot, c: &Entity, d: f64) -> Option<Bid> {
    let me = &snap.me;
    let face = Commands::default().look_at(c.head());
    if c.state.fuse_lit && c.state.swell >= FLEE_SWELL {
        return None;
    }
    let slot = sword_slot(snap)?;
    if me.selected_slot != slot {
        let cmds = face.main(MainCmd::SelectSlot(slot));
        return Some(Bid::new(Priority::P2, 0.8, "creeper_draw", cmds));
    }
    let ready = me.attack_ready >= 0.95;
    let away = safe_retreat(snap, c.pos);
    let toward = heading_to(me.pos, c.pos);
    let urgent = if d <= BLAST {
        Priority::P0
    } else {
        Priority::P2
    };
    if ready && d <= STRIKE {
        // Sprinting into the swing makes it a sprint hit with the extra knockback.
        let cmds = face
            .main(MainCmd::Attack(c.id))
            .legs(LegsCmd::toward(toward, Speed::Sprint));
        return Some(Bid::new(urgent, 1.0, "creeper_strike", cmds));
    }
    if c.state.fuse_lit || d < KEEP - 2.0 || !ready {
        // Recharge out of fuse range: past 7 a lit fuse burns back down.
        if d > KEEP + 1.0 && !c.state.fuse_lit {
            return Some(Bid::new(Priority::P2, 0.3, "creeper_wait", face));
        }
        let cmds = face.legs(LegsCmd::toward(away, Speed::Sprint));
        return Some(Bid::new(urgent, 1.0, "creeper_recoil", cmds));
    }
    let cmds = face.legs(LegsCmd::toward(toward, Speed::Sprint));
    Some(Bid::new(Priority::P2, 0.8, "creeper_engage", cmds))
}

fn stance_bid(ctx: &Ctx, c: &Entity, d: f64) -> Option<Bid> {
    let snap = ctx.snap;
    let me = &snap.me;
    let approaching = tau(me.pos, me.vel, c.pos, c.vel).is_some();
    let away = safe_retreat(snap, c.pos);
    let face = Commands::default().look_at(c.head());
    if c.state.fuse_lit && d < KEEP {
        let priority = if d <= BLAST {
            Priority::P0
        } else {
            Priority::P2
        };
        if me.offhand.is_some_and(|s| s.kind == ItemKind::Shield) {
            let cmds = face
                .off(OffCmd::RaiseShield)
                .legs(LegsCmd::toward(away, Speed::Walk));
            return Some(Bid::new(priority, 1.0, "creeper_shield", cmds).lock(5));
        }
        let cmds = Commands::default().legs(LegsCmd::toward(away, Speed::Sprint));
        return Some(Bid::new(priority, 1.0, "creeper_flee", cmds));
    }
    if d >= KEEP && !approaching {
        return None;
    }
    let urgency = (1.0 - (d - BLAST) / (WATCH - BLAST)).clamp(0.2, 1.0) as f32;
    let legs = LegsCmd::toward(away, Speed::Walk);
    let full = face.legs(legs);
    let degraded = Commands::default().legs(legs);
    Some(Bid::new(Priority::P2, urgency, "creeper_back_away", full).degraded(degraded))
}
