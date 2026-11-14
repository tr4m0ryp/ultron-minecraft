use crate::behaviours::common::heading_to;
use crate::channels::{Commands, LegsCmd, MainCmd, Speed};
use crate::goals::ports::NavTarget;
use crate::goals::task::{GoalCx, Plan};
use crate::goals::travel::{Walk, Walker};
use crate::snapshot::{Entity, ItemKind};

/// Strike from inside this distance (survival entity reach is 3).
pub const MELEE_REACH: f64 = 2.9;
/// Beyond this the planner closes the distance; inside it the goal steers directly.
const DIRECT: f64 = 6.0;
const READY: f32 = 0.9;

/// One tick of closing in on `target` and striking when the cooldown allows. Returns the
/// plan and whether it swings this tick.
pub fn engage(cx: &mut GoalCx, walker: &mut Walker, target: &Entity) -> (Plan, bool) {
    let me = &cx.snap().me;
    let d = target.pos.dist(me.pos);
    if d > DIRECT {
        let t = NavTarget::Entity {
            id: target.id,
            distance: MELEE_REACH,
        };
        return match walker.walk(cx, t, "goal_chase") {
            Walk::Moving(p) => (p, false),
            Walk::Arrived => (Plan::hold("goal_chase"), false),
            Walk::Failed(f) => {
                cx.blocked(format!("cannot reach the {}: {f:?}", target.kind.label()));
                (Plan::hold("goal_chase"), false)
            }
        };
    }
    walker.stop(cx);
    let me = &cx.snap().me;
    let legs = if d > MELEE_REACH - 0.4 {
        LegsCmd::toward(heading_to(me.pos, target.pos), Speed::Walk)
    } else {
        LegsCmd::stop()
    };
    let mut c = Commands::default().legs(legs).look_at(target.head());
    let weapon = me
        .hotbar_find(|k| k == ItemKind::Sword)
        .or_else(|| me.hotbar_find(|k| k == ItemKind::Axe));
    let mut swing = false;
    match weapon {
        Some(s) if s != me.selected_slot => c = c.main(MainCmd::SelectSlot(s)),
        _ if d <= MELEE_REACH && me.attack_ready >= READY => {
            c = c.main(MainCmd::Attack(target.id));
            swing = true;
        }
        _ => {}
    }
    let degraded = Commands::default().legs(legs);
    (Plan::new("goal_melee", c).degraded(degraded), swing)
}
