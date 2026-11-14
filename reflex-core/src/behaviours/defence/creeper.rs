use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::behaviours::common::safe_retreat;
use crate::channels::{Commands, LegsCmd, MainCmd, OffCmd, Speed};
use crate::predict::tau;
use crate::snapshot::{EntityKind, ItemKind};

/// D1: creeper within 16 and approaching: back away to keep 7 blocks while facing it.
/// Fuse lit within 7: raise a held shield, else sprint straight away (P2, P0 within 3).
/// With aggression above 0.7 and the attack ready within 3: one knockback hit first.
#[derive(Default)]
pub struct CreeperStance;

const WATCH: f64 = 16.0;
const KEEP: f64 = 7.0;
const BLAST: f64 = 3.0;

impl Behaviour for CreeperStance {
    fn id(&self) -> &'static str {
        "D1"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let snap = ctx.snap;
        let me = &snap.me;
        let c = snap
            .nearby(WATCH, |e| e.kind == EntityKind::Creeper)
            .into_iter()
            .next()?;
        let d = c.pos.dist(me.pos);
        let approaching = tau(me.pos, me.vel, c.pos, c.vel).is_some();
        let away = safe_retreat(snap, c.pos);
        let face = Commands::default().look_at(c.head());

        if c.state.fuse_lit && d < KEEP {
            let priority = if d <= BLAST {
                Priority::P0
            } else {
                Priority::P2
            };
            let a = ctx.knobs.gains.aggression;
            if a > 0.7 && d <= BLAST && me.attack_ready >= 0.9 {
                let cmds = face
                    .main(MainCmd::Attack(c.id))
                    .legs(LegsCmd::toward(away, Speed::Walk));
                return Some(Bid::new(priority, 1.0, "creeper_knockback", cmds));
            }
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
}
