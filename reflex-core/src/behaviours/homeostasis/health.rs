use super::eat::{best_food, eat_commands, EATING_TICKS};
use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::behaviours::common::{heading_to, safe_retreat};
use crate::channels::{Commands, LegsCmd, MainCmd, Speed};

/// H2: hp under hp_floor with a hostile within 16: retreat (toward the companion when it is
/// farther from the threat than we are). hp 6 or less with a golden apple and no hostile
/// within 5: eat it. hp < 20 and food < 18 and no hostile within 12: top off food (P3).
#[derive(Default)]
pub struct Health;

const GOLDEN_HP: f32 = 6.0;

impl Behaviour for Health {
    fn id(&self) -> &'static str {
        "H2"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let snap = ctx.snap;
        let me = &snap.me;
        let floor = ctx.knobs.gains.hp_floor;
        let threat = snap.nearest_hostile(16.0);
        let mut cmds = Commands::default();
        let mut priority = None;
        let mut lock = 0;

        if let (Some(t), true) = (threat, me.health < floor) {
            let to_companion = snap
                .companion
                .as_ref()
                .filter(|c| c.pos.dist(t.pos) > me.pos.dist(t.pos))
                .map(|c| heading_to(me.pos, c.pos));
            let heading = to_companion.unwrap_or_else(|| safe_retreat(snap, t.pos));
            cmds = cmds.legs(LegsCmd::toward(heading, Speed::Sprint));
            priority = Some(Priority::P1);
        }
        if me.health <= GOLDEN_HP && threat.is_some() && snap.nearest_hostile(5.0).is_none() {
            if let Some(slot) = best_food(me, true) {
                let eat = eat_commands(me, slot);
                lock = if eat.main == Some(MainCmd::Use) {
                    EATING_TICKS
                } else {
                    0
                };
                cmds.main = eat.main;
                cmds.inv = eat.inv;
                priority = Some(Priority::P1);
            }
        }
        if priority.is_none()
            && me.health < 20.0
            && me.food < 18
            && snap.nearest_hostile(12.0).is_none()
        {
            let slot = best_food(me, false)?;
            let eat = eat_commands(me, slot);
            lock = if eat.main == Some(MainCmd::Use) {
                EATING_TICKS
            } else {
                0
            };
            return Some(Bid::new(Priority::P3, 0.5, "top_off_food", eat).lock(lock));
        }
        let priority = priority?;
        let urgency = 1.0 - me.health / floor.max(1.0);
        Some(Bid::new(priority, urgency.max(0.1), "low_health", cmds).lock(lock))
    }
}
