use crate::behaviour::{Behaviour, Bid, Ctx, Priority, Veto};
use crate::channels::{Commands, LegsCmd, Speed};
use crate::events::EventClass;
use crate::predict::{expected_step, fall_damage};
use crate::snapshot::BlockKind;

/// What lies under the next footstep the plan would take. A landing of `Unknown` means the
/// sensed column ended; `depth` is then a lower bound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drop {
    pub depth: f64,
    pub landing: BlockKind,
    pub void: bool,
}

/// Lookahead in blocks beyond the commanded step, so the stop lands before the edge.
const LOOKAHEAD: f64 = 0.45;
/// Full height of the tallest dimension; columns are hash lookups, so scanning to the void
/// is cheap.
const SCAN_DEPTH: i32 = 512;

/// Probes the cell the legs command would carry us into next. `None` if the step keeps
/// support (or there is no step).
pub fn probe_drop(ctx: &Ctx, legs: &LegsCmd) -> Option<Drop> {
    let me = &ctx.snap.me;
    if !me.on_ground || legs.speed == Speed::Stop || legs.heading.horizontal().len() < 1e-6 {
        return None;
    }
    let step = expected_step(legs);
    let next = me.pos + step + legs.heading.horizontal().normalized() * LOOKAHEAD;
    let cell = next.block();
    let blocks = &ctx.snap.blocks;
    if !blocks.get(cell).passable() || blocks.get(cell.below()).supports() {
        return None;
    }
    let feet_y = me.pos.block().y;
    let depth_to_void = cell.below().y - ctx.snap.world_min_y;
    match blocks.floor_below(cell.below(), SCAN_DEPTH.min(depth_to_void)) {
        Some((p, k)) => {
            // An unknown cell bounds the depth from below: the drop is at least this deep.
            let top = if k.supports() { p.y + 1 } else { p.y };
            Some(Drop { depth: (feet_y - top) as f64, landing: k, void: false })
        }
        None if depth_to_void <= SCAN_DEPTH => {
            Some(Drop { depth: f64::INFINITY, landing: BlockKind::Air, void: true })
        }
        None => Some(Drop { depth: f64::INFINITY, landing: BlockKind::Unknown, void: false }),
    }
}

fn halt(plan: &mut Commands) {
    if let Some(l) = plan.legs.as_mut() {
        l.speed = Speed::Sneak;
        l.heading = crate::math::Vec3::ZERO;
        l.jump = false;
    }
}

/// R5: the next step drops deeper than safe_drop (3 blocks at caution 0, 1 at caution 1)
/// or into lava. Sneak and zero the forward input. Escalates to P0 (uninhibitable) when
/// the landing would be lethal with no mitigation.
#[derive(Default)]
pub struct EdgeGuard {
    reported_goal: Option<u64>,
}

impl Behaviour for EdgeGuard {
    fn id(&self) -> &'static str {
        "R5"
    }

    fn evaluate(&mut self, _ctx: &Ctx) -> Option<Bid> {
        None
    }

    fn veto(&mut self, ctx: &Ctx, plan: &mut Commands) -> Option<Veto> {
        let drop = probe_drop(ctx, plan.legs.as_ref()?)?;
        if drop.void {
            return None;
        }
        let lava = drop.landing == BlockKind::Lava;
        let mitigated = drop.landing.negates_fall();
        if drop.depth <= ctx.knobs.gains.safe_drop() && !lava {
            return None;
        }
        if mitigated && !lava {
            return None;
        }
        let me = &ctx.snap.me;
        let lethal = lava || fall_damage(drop.depth) >= me.hp_total();
        halt(plan);
        if let Some(g) = ctx.knobs.goal.as_ref().filter(|g| self.reported_goal != Some(g.id)) {
            self.reported_goal = Some(g.id);
            ctx.emit(ctx.report(EventClass::Digest, "goal_blocked_by_edge", "R5").text(g.spec.clone()));
        }
        let priority = if lethal { Priority::P0 } else { Priority::P1 };
        Some(Veto { priority, note: "edge_guard" })
    }
}

/// R7: an edge whose column has no blocks down to the void, or standing near the world
/// floor. Treated as an infinite drop and never inhibited.
#[derive(Default)]
pub struct VoidGuard {
    warned: bool,
}

impl Behaviour for VoidGuard {
    fn id(&self) -> &'static str {
        "R7"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let low = ctx.snap.me.pos.y < (ctx.snap.world_min_y + 4) as f64;
        if low && !self.warned {
            ctx.emit(ctx.report(EventClass::Wake, "near_void", "R7").critical().text("below world floor + 4"));
        }
        self.warned = low;
        None
    }

    fn veto(&mut self, ctx: &Ctx, plan: &mut Commands) -> Option<Veto> {
        let drop = probe_drop(ctx, plan.legs.as_ref()?)?;
        if !drop.void {
            return None;
        }
        halt(plan);
        Some(Veto { priority: Priority::P0, note: "void_guard" })
    }
}
