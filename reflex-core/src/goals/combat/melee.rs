use crate::behaviours::defence::melee_commands;
use crate::channels::{Commands, MainCmd};
use crate::goals::ports::NavTarget;
use crate::goals::task::{GoalCx, Plan};
use crate::goals::travel::{Walk, Walker};
use crate::snapshot::{Entity, ItemKind};

/// Strike from inside this distance (survival entity reach is 3).
pub const MELEE_REACH: f64 = 2.9;
/// Beyond this the planner closes the distance; inside it the goal steers directly.
const DIRECT: f64 = 6.0;

/// One tick of closing in on `target` and striking when the cooldown allows: the planner
/// brings the body within `DIRECT`, then the K1 melee micro takes over. Returns the plan
/// and whether it swings this tick.
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
                cx.stuck(format!("cannot reach the {}: {f:?}", target.kind.label()));
                (Plan::hold("goal_chase"), false)
            }
        };
    }
    walker.stop(cx);
    let snap = cx.snap();
    // Close range is the K1 micro the reflex D2 uses, so the goal's blows land exactly as
    // the reflex's do. Live, the goal's own swing never killed: its fallback plan dropped
    // the strike, and orienting (O2, P2) takes the gaze from a P5 goal whenever a mob
    // closes, so the full plan lost and the fallback ran without the attack.
    let me = &snap.me;
    let cmds = match (
        me.hotbar_find(|k| k == ItemKind::Sword),
        me.hotbar_find(|k| k == ItemKind::Axe),
    ) {
        (None, Some(slot)) if slot != me.selected_slot => Commands::default()
            .look_at(target.head())
            .main(MainCmd::SelectSlot(slot)),
        _ => melee_commands(snap, target),
    };
    let swing = matches!(cmds.main, Some(MainCmd::Attack(_)));
    let mut degraded = cmds.clone();
    degraded.gaze = None;
    (Plan::new("goal_melee", cmds).degraded(degraded), swing)
}
