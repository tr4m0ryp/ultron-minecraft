use super::melee::melee_commands;
use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::behaviours::common::{clearance, open_retreat};
use crate::channels::{Commands, LegsCmd, MainCmd, OffCmd, Speed};
use crate::math::Vec3;
use crate::predict::cpa;
use crate::snapshot::{EntityClass, EntityKind, ItemKind};

const SHIELD_RAISE_TICKS: f64 = 5.0;
const HORIZON: f64 = 15.0;

fn shield_in_offhand(ctx: &Ctx) -> bool {
    ctx.snap
        .me
        .offhand
        .is_some_and(|s| s.kind == ItemKind::Shield)
}

/// R14 with K2: a projectile whose closest approach passes within 1 block (wider with
/// caution) in at most 15 ticks. Raise the shield if there are at least 5 ticks left (it
/// blocks only after 5), else strafe perpendicular to its flight. Hit a ghast fireball
/// back when it is within reach and 4 ticks out.
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
        if shield_in_offhand(ctx) && t >= SHIELD_RAISE_TICKS {
            return Some(
                Bid::new(
                    Priority::P1,
                    1.0,
                    "shield_block",
                    face.off(OffCmd::RaiseShield),
                )
                .lock(5),
            );
        }
        let v = p.vel.horizontal().normalized();
        let side = Vec3::new(-v.z, 0.0, v.x);
        let cmds = Commands::default().legs(LegsCmd::toward(side, Speed::Sprint));
        Some(Bid::new(Priority::P1, 1.0, "strafe_projectile", cmds))
    }
}

/// D3: skeleton, stray or bogged within 16. With aggression >= 0.6 close in with K1,
/// shield raised while it draws. Otherwise retreat out of its 16-block range, raising the
/// shield on the draw, but only when retreating can work: trapped (no open retreat
/// heading, or a wall or drop within 3 blocks along it) or with the archer already within
/// 6 blocks, close in and kill it instead, as a skilled player does in a closed room. Live
/// in a walled 30x30 arena the pure retreat ran along the walls and was shot to death 5
/// times in 225 s without a swing. Line of sight is not computed: every archer in range
/// counts.
#[derive(Default)]
pub struct RangedCover;

const RANGE: f64 = 16.0;
/// Within this an archer hits nearly every shot and is about a second's sprint away.
const CLOSE_IN: f64 = 6.0;
/// A retreat needs this much open ground ahead to be worth running.
const ESCAPE_ROOM: f64 = 3.0;

impl Behaviour for RangedCover {
    fn id(&self) -> &'static str {
        "D3"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let snap = ctx.snap;
        let me = &snap.me;
        let archer = snap
            .nearby(RANGE, |e| {
                e.kind.is_ranged() || (e.kind == EntityKind::Drowned && e.state.holding_trident)
            })
            .into_iter()
            .next()?;
        let d = archer.pos.dist(me.pos);
        let urgency = (1.0 - d / RANGE).clamp(0.2, 1.0) as f32;
        let block = archer.state.drawing_bow && shield_in_offhand(ctx);
        let retreat = open_retreat(snap, archer.pos)
            .filter(|&h| clearance(&snap.blocks, me.pos, h, ESCAPE_ROOM).0 >= ESCAPE_ROOM);
        let trapped = retreat.is_none() || d <= CLOSE_IN;
        if ctx.knobs.gains.aggression >= 0.6 || trapped {
            let mut cmds = melee_commands(snap, archer);
            if block && cmds.main.is_none() {
                cmds = cmds.off(OffCmd::RaiseShield);
            }
            let note = if trapped {
                "ranged_trapped_close_in"
            } else {
                "ranged_close_in"
            };
            let mut degraded = cmds.clone();
            degraded.gaze = None;
            return Some(Bid::new(Priority::P2, urgency, note, cmds).degraded(degraded));
        }
        let heading = retreat?;
        let mut cmds = Commands::default().legs(LegsCmd::toward(heading, Speed::Sprint));
        if block {
            cmds = cmds.off(OffCmd::RaiseShield).look_at(archer.head());
            if let Some(l) = cmds.legs.as_mut() {
                l.speed = Speed::Walk;
            }
        }
        Some(Bid::new(Priority::P2, urgency, "ranged_cover", cmds))
    }
}
