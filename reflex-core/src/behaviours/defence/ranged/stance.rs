use super::cadence::{exposed_to, ticks_to_hit, RANGE};
use super::moves::Footwork;
use super::shield_in_offhand;
use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::behaviours::common::{clearance, cover_spot, heading_to, open_retreat};
use crate::behaviours::defence::melee_commands;
use crate::channels::{Commands, LegsCmd, OffCmd, Speed};
use crate::math::Vec3;
use crate::snapshot::{Entity, EntityKind};

/// Raise the shield when an arrow could land within this many ticks: 5 to raise, plus
/// margin for seeing the draw a tick or two late.
const GUARD_TICKS: f64 = 10.0;
/// Keep the sideways speed through a release expected within this many ticks.
const HOLD_TICKS: f64 = 14.0;
const REACH: f64 = 3.0;
/// Within this an archer hits nearly every shot and is about a second's sprint away.
const CLOSE_IN: f64 = 6.0;
/// A retreat needs this much open ground ahead to be worth running.
const ESCAPE_ROOM: f64 = 3.0;
/// Health above hp_floor before recovery mode ends (hysteresis).
const RECOVER_MARGIN: f32 = 6.0;
/// Zigzag angle off the line to the archer while closing in; sideways while recovering.
const ZIG_DEG: f64 = 50.0;
const CIRCLE_DEG: f64 = 90.0;
/// Without a shield, this many archers in sight send us to cover when there is any.
const CROWD: usize = 3;

/// D3: skeleton, stray or bogged within 16, played as a skilled player does. Fight
/// (aggression >= 0.5, or nowhere to run): with a shield, sprint in during the archer's
/// reload and raise the shield toward the archer whose arrow is due within 10 ticks
/// (5 to raise plus margin), lowering it to strike; without one, zigzag in so the arrow,
/// aimed with no lead, passes beside us. Below hp_floor (until 6 above it), or without a
/// shield against three archers in sight: break line of sight at the nearest cover spot and
/// hold it; in the open, circle sideways at a walk (sprinting would burn the food that
/// regenerates health) with the shield up when an arrow is due. Low aggression on open
/// ground keeps the old retreat out of range.
#[derive(Default)]
pub struct RangedCover {
    foot: Footwork,
    recovering: bool,
}

impl Behaviour for RangedCover {
    fn id(&self) -> &'static str {
        "D3"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let snap = ctx.snap;
        let me = &snap.me;
        let archers = snap.nearby(RANGE, |e| {
            e.kind.is_ranged() || (e.kind == EntityKind::Drowned && e.state.holding_trident)
        });
        let exposed = exposed_to(snap);
        let target = exposed.first().or(archers.first()).copied()?;
        let g = &ctx.knobs.gains;
        if me.health < g.hp_floor {
            self.recovering = true;
        } else if me.health >= g.hp_floor + RECOVER_MARGIN {
            self.recovering = false;
        }
        let shield = shield_in_offhand(ctx);
        let soonest = exposed
            .iter()
            .map(|e| (ticks_to_hit(e, me.pos), *e))
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let due = |ticks: f64| soonest.is_some_and(|(t, _)| t <= ticks);
        let guard = shield && due(GUARD_TICKS);
        let d = target.pos.dist(me.pos);
        let urgency = (1.0 - d / RANGE).clamp(0.2, 1.0) as f32;
        let toward = heading_to(me.pos, target.pos);

        if self.recovering || (!shield && exposed.len() >= CROWD) {
            if exposed.is_empty() {
                let hold = Commands::default().legs(LegsCmd::stop());
                return Some(Bid::new(Priority::P2, 0.4, "ranged_hold_cover", hold));
            }
            let eyes: Vec<Vec3> = exposed.iter().map(|e| e.head()).collect();
            if let Some(spot) = cover_spot(snap, &eyes) {
                let at = spot.feet_centre();
                let legs = if at.dist(me.pos) < 0.3 {
                    LegsCmd::stop()
                } else {
                    LegsCmd::toward(heading_to(me.pos, at), Speed::Sprint)
                };
                let cmds = Commands::default().legs(legs);
                return Some(Bid::new(Priority::P2, 1.0, "ranged_take_cover", cmds));
            }
        }
        if self.recovering && d > REACH {
            let h = self.foot.heading(snap, toward, CIRCLE_DEG, due(HOLD_TICKS));
            let mut cmds = Commands::default()
                .look_at(target.head())
                .legs(LegsCmd::toward(h, Speed::Walk));
            if let (true, Some((_, e))) = (guard, soonest) {
                cmds = cmds.look_at(e.head()).off(OffCmd::RaiseShield);
            }
            return Some(Bid::new(Priority::P2, 1.0, "ranged_evade", cmds));
        }

        let retreat = open_retreat(snap, target.pos)
            .filter(|&h| clearance(&snap.blocks, me.pos, h, ESCAPE_ROOM).0 >= ESCAPE_ROOM);
        let trapped = retreat.is_none() || d <= CLOSE_IN;
        if g.aggression < 0.5 && !trapped {
            return retreat.map(|h| self.retreat(target, h, guard, urgency));
        }
        let strike = d <= REACH && me.attack_ready >= 0.95;
        if let (true, false, Some((_, e))) = (guard, strike, soonest) {
            let cmds = Commands::default()
                .look_at(e.head())
                .off(OffCmd::RaiseShield)
                .legs(LegsCmd::toward(toward, Speed::Walk));
            return Some(Bid::new(Priority::P2, 1.0, "ranged_shield_advance", cmds));
        }
        let mut cmds = melee_commands(snap, target);
        let note = if !shield && d > REACH {
            let h = self.foot.heading(snap, toward, ZIG_DEG, due(HOLD_TICKS));
            cmds.legs = Some(LegsCmd::toward(h, Speed::Sprint));
            "ranged_zigzag"
        } else if trapped {
            "ranged_trapped_close_in"
        } else {
            "ranged_close_in"
        };
        let mut degraded = cmds.clone();
        degraded.gaze = None;
        Some(Bid::new(Priority::P2, urgency, note, cmds).degraded(degraded))
    }
}

impl RangedCover {
    /// Run out of range on open ground, walking with the shield up when an arrow is due.
    fn retreat(&self, archer: &Entity, heading: Vec3, guard: bool, urgency: f32) -> Bid {
        let mut cmds = Commands::default().legs(LegsCmd::toward(heading, Speed::Sprint));
        if guard {
            cmds = cmds
                .off(OffCmd::RaiseShield)
                .look_at(archer.head())
                .legs(LegsCmd::toward(heading, Speed::Walk));
        }
        Bid::new(Priority::P2, urgency, "ranged_cover", cmds)
    }
}
