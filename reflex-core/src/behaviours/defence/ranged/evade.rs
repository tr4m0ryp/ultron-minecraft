use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::channels::{Commands, LegsCmd, MainCmd, OffCmd, Speed};
use crate::math::Vec3;
use crate::predict::cpa;
use crate::snapshot::EntityClass;
use crate::snapshot::EntityKind;

use super::shield_in_offhand;

/// A raised shield blocks only from its 5th tick.
pub const SHIELD_RAISE_TICKS: u32 = 5;
const HORIZON: f64 = 15.0;

/// R14 with K2: a projectile whose closest approach passes within 1 block (wider with
/// caution) in at most 15 ticks. Keep a shield that is already up raised and facing it;
/// raise one if there are at least 5 ticks left (it blocks only after 5), else strafe
/// perpendicular to its flight. Hit a ghast fireball back when it is within reach and 4
/// ticks out.
#[derive(Default)]
pub struct ProjectileEvade;

impl Behaviour for ProjectileEvade {
    fn id(&self) -> &'static str {
        "R14"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let snap = ctx.snap;
        let me = &snap.me;
        let centre = me.pos + Vec3::new(0.0, 0.9, 0.0);
        let miss_max = 1.0 + ctx.knobs.gains.caution as f64 * 0.5;
        let (p, t) = snap
            .entities
            .iter()
            .filter(|e| e.kind.class() == EntityClass::Projectile && !e.state.inert)
            .filter_map(|e| {
                let (miss, t) = cpa(centre, e.pos, e.vel - me.vel);
                (miss < miss_max && t > 0.0 && t <= HORIZON).then_some((e, t))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))?;
        let face = Commands::default().look_at(p.pos);
        if p.kind == EntityKind::Fireball && p.pos.dist(me.eye()) <= 3.0 && t <= 4.0 {
            return Some(Bid::new(
                Priority::P1,
                1.0,
                "deflect_fireball",
                face.main(MainCmd::Attack(p.id)),
            ));
        }
        let raised = me.shield_ticks > 0;
        if shield_in_offhand(ctx) && (raised || t >= SHIELD_RAISE_TICKS as f64) {
            let bid = Bid::new(
                Priority::P1,
                1.0,
                "shield_block",
                face.off(OffCmd::RaiseShield),
            );
            return Some(bid.lock(SHIELD_RAISE_TICKS));
        }
        let v = p.vel.horizontal().normalized();
        let side = Vec3::new(-v.z, 0.0, v.x);
        let cmds = Commands::default().legs(LegsCmd::toward(side, Speed::Sprint));
        Some(Bid::new(Priority::P1, 1.0, "strafe_projectile", cmds))
    }
}
