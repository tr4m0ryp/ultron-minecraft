use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::behaviours::common::heading_to;
use crate::channels::{Commands, LegsCmd, Speed};
use crate::events::EventClass;
use crate::math::Vec3;
use crate::snapshot::EntityClass;

/// H8: attachment homeostat. Danger to the companion (hostile within 4 of them, or their
/// health dropping) brings us to them at P2 and wakes the brain. Beyond the leash we walk
/// back to within 3 blocks (P4); beyond 32 the brain is woken. Standing within 1 block while
/// they walk toward us, we step aside. When a goal ends the follow holds off for
/// [`GOAL_GRACE`] ticks, so the body stays where the goal left it (its drops, the spot the
/// brain's next goal starts from) until the woken brain has issued that next goal.
#[derive(Default)]
pub struct CompanionLeash {
    following: bool,
    last_hp: Option<f32>,
    danger_reported: bool,
    far_reported: bool,
    had_goal: bool,
    grace_until: u64,
}

const RETURN_TO: f64 = 3.0;
const FAR: f64 = 32.0;
/// Ticks after a goal ends before the straight-line follow may resume (10 s).
pub const GOAL_GRACE: u64 = 200;

impl Behaviour for CompanionLeash {
    fn id(&self) -> &'static str {
        "H8"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let snap = ctx.snap;
        let c = snap.companion.as_ref()?;
        let me = &snap.me;
        let dist = me.pos.dist(c.pos);
        let hp_drop = matches!((self.last_hp, c.health), (Some(a), Some(b)) if b < a);
        self.last_hp = c.health;
        let hostile_near = snap
            .entities
            .iter()
            .any(|e| e.kind.class() == EntityClass::Hostile && e.pos.dist(c.pos) <= 4.0);

        let danger = hostile_near || hp_drop;
        if danger && !self.danger_reported {
            ctx.emit(
                ctx.report(EventClass::Wake, "companion_danger", &c.name)
                    .text("hostile near companion or companion hurt"),
            );
        }
        self.danger_reported = danger;
        let far = dist > FAR;
        if far && !self.far_reported {
            ctx.emit(
                ctx.report(EventClass::Wake, "companion_far", &c.name)
                    .magnitude(dist as f32),
            );
        }
        self.far_reported = far;

        if danger && dist > 2.0 {
            let legs = LegsCmd::toward(heading_to(me.pos, c.pos), Speed::Sprint);
            return Some(Bid::new(
                Priority::P2,
                1.0,
                "protect_companion",
                Commands::default().legs(legs),
            ));
        }

        let solo = ctx.knobs.goal.as_ref().is_some_and(|g| {
            g.solo || crate::goals::parse(&g.spec, g.target).is_ok_and(|s| s.tracks_companion())
        });
        let has_goal = ctx.knobs.goal.is_some();
        if self.had_goal && !has_goal {
            self.grace_until = snap.tick + GOAL_GRACE;
        }
        self.had_goal = has_goal;
        let leash = ctx.knobs.gains.leash;
        if leash <= 0.0 || solo || snap.tick < self.grace_until {
            self.following = false;
            return None;
        }
        if dist > leash {
            self.following = true;
        } else if dist <= RETURN_TO {
            self.following = false;
        }
        // A wall at head height on the straight line means walking at the companion only
        // presses the body into it; leave the legs to the escape drive or the next goal.
        if self.following && head_blocked(snap, me.pos, c.pos) {
            return None;
        }
        if self.following {
            let speed = if dist > 2.0 * leash && me.food > 6 {
                Speed::Sprint
            } else {
                Speed::Walk
            };
            let urgency = ((dist - RETURN_TO) / leash).clamp(0.2, 1.0) as f32;
            let mut legs = LegsCmd::toward(heading_to(me.pos, c.pos), speed);
            // Step up a one-block rise on the way, as a player would.
            legs.jump = me.on_ground && step_ahead(snap, me.pos, c.pos);
            return Some(Bid::new(
                Priority::P4,
                urgency,
                "follow_companion",
                Commands::default().legs(legs),
            ));
        }
        let toward_us = c.vel.horizontal().dot(me.pos - c.pos) > 0.01;
        if dist < 1.0 && toward_us {
            let v = c.vel.horizontal().normalized();
            let aside = Vec3::new(-v.z, 0.0, v.x);
            let legs = LegsCmd::toward(aside, Speed::Walk);
            return Some(Bid::new(
                Priority::P4,
                0.5,
                "step_aside",
                Commands::default().legs(legs),
            ));
        }
        None
    }
}

/// The cells at head height in the next two blocks toward `to` are solid.
fn head_blocked(snap: &crate::snapshot::Snapshot, from: Vec3, to: Vec3) -> bool {
    let dir = Vec3::new(to.x - from.x, 0.0, to.z - from.z).normalized();
    let head = from.y.floor() + 1.5;
    [0.8, 1.6].iter().all(|&d| {
        let p = from + dir * d;
        let k = snap.blocks.get(Vec3::new(p.x, head, p.z).block());
        k != crate::snapshot::BlockKind::Unknown && !k.passable()
    })
}

/// A block at feet height right ahead toward `to` with room above it: a step to jump.
fn step_ahead(snap: &crate::snapshot::Snapshot, from: Vec3, to: Vec3) -> bool {
    let dir = Vec3::new(to.x - from.x, 0.0, to.z - from.z).normalized();
    let p = from + dir * 0.7;
    let feet = Vec3::new(p.x, from.y.floor() + 0.5, p.z).block();
    !snap.blocks.get(feet).passable()
        && snap.blocks.get(feet.above()).passable()
        && snap.blocks.get(feet.above().above()).passable()
}
