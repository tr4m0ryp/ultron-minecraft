//! Brain-armed if-then responses, evaluated every tick without the brain (StartReact).

use crate::behaviour::{Bid, Ctx, Priority};
use crate::behaviours::common::{heading_to, safe_retreat};
use crate::channels::{Commands, LegsCmd, MainCmd, Speed};
use crate::knobs::{Knobs, Response, Trigger};
use crate::selector::Candidate;
use crate::snapshot::{Entity, ItemKind, Snapshot};

fn nearest_of<'a>(snap: &'a Snapshot, kind: crate::snapshot::EntityKind, from: crate::math::Vec3, d: f64) -> Option<&'a Entity> {
    snap.entities
        .iter()
        .filter(|e| e.kind == kind && e.pos.dist(from) <= d)
        .min_by(|a, b| a.pos.dist(from).total_cmp(&b.pos.dist(from)))
}

fn triggered(ctx: &Ctx, t: &Trigger) -> bool {
    let snap = ctx.snap;
    match t {
        Trigger::EntityWithin { kind, dist, of_companion } => {
            let from = match (of_companion, &snap.companion) {
                (true, Some(c)) => c.pos,
                (true, None) => return false,
                (false, _) => snap.me.pos,
            };
            nearest_of(snap, *kind, from, *dist).is_some()
        }
        Trigger::HpBelow(x) => snap.me.health < *x,
        Trigger::CompanionHpBelow(x) => {
            snap.companion.as_ref().and_then(|c| c.health).is_some_and(|h| h < *x)
        }
        Trigger::ChatMatches(p) => {
            let p = p.to_lowercase();
            snap.chat.iter().any(|c| c.text.to_lowercase().contains(&p))
        }
    }
}

fn respond(ctx: &Ctx, r: &Response) -> Option<Commands> {
    let snap = ctx.snap;
    let me = snap.me.pos;
    Some(match r {
        Response::Say(text) => Commands::default().chat(text.clone()),
        Response::RetreatTo(p) => Commands::default().legs(LegsCmd::toward(heading_to(me, *p), Speed::Sprint)),
        Response::FleeFrom(kind) => {
            let e = nearest_of(snap, *kind, me, 32.0)?;
            Commands::default()
                .legs(LegsCmd::toward(safe_retreat(snap, e.pos), Speed::Sprint))
                .look_at(e.head())
        }
        Response::Attack(kind) => {
            let e = nearest_of(snap, *kind, me, 32.0)?;
            let cmds = Commands::default().look_at(e.head());
            if e.pos.dist(me) <= 3.0 && snap.me.attack_ready >= 0.9 {
                cmds.main(MainCmd::Attack(e.id))
            } else {
                cmds.legs(LegsCmd::toward(heading_to(me, e.pos), Speed::Sprint))
            }
        }
        Response::PlaceBlockBetween(kind) => {
            let comp = snap.companion.as_ref()?;
            let e = nearest_of(snap, *kind, comp.pos, 32.0)?;
            let mid = (e.pos + comp.pos) * 0.5;
            let target = mid.block().below();
            let cmds = Commands::default().look_at(target.feet_centre());
            match snap.me.held() {
                Some(s) if s.kind == ItemKind::ThrowawayBlock => cmds.main(MainCmd::PlaceOn(target)),
                _ => {
                    let slot = snap.me.hotbar_find(|k| k == ItemKind::ThrowawayBlock)?;
                    cmds.main(MainCmd::SelectSlot(slot))
                }
            }
        }
    })
}

/// Bids for every armed response whose trigger holds this tick. They run at P2.
pub fn bids(ctx: &Ctx) -> Vec<(u64, Bid)> {
    ctx.knobs
        .armed
        .iter()
        .filter(|a| triggered(ctx, &a.trigger))
        .filter_map(|a| {
            let cmds = respond(ctx, &a.response)?;
            Some((a.id, Bid::new(Priority::P2, 1.0, "armed", cmds)))
        })
        .collect()
}

/// Drops one-shot armed responses that won a channel this tick.
pub fn retire_fired(knobs: &mut Knobs, cands: &[Candidate], winners: &[usize]) {
    let fired: Vec<u64> = winners
        .iter()
        .filter_map(|&w| cands[w].id.strip_prefix("ARM:")?.parse().ok())
        .collect();
    knobs.armed.retain(|a| !(a.once && fired.contains(&a.id)));
}
