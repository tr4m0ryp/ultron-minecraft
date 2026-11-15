mod jolt;
mod scrape;
mod work;

pub use work::scaffold_slot;

use crate::behaviours::common::heading_to;
use crate::channels::{Commands, LegsCmd, Speed};
use crate::goals::ports::{NavFailure, NavStatus, NavTarget};
use crate::goals::task::{GoalCx, Plan};
use crate::math::{BlockPos, Vec3};
use jolt::Jolt;
use scrape::Scrape;

/// Planner failures in a row before a walk gives up.
pub const MAX_NAV_FAILURES: u32 = 3;
/// Ticks of planning (not pre-empted) before it counts as a failure.
const PLANNING_LIMIT: u32 = 400;

// One value per goal per tick; boxing the plan would buy nothing.
#[allow(clippy::large_enum_variant)]
pub enum Walk {
    Arrived,
    /// Bid this plan: a legs command along the path, or standing still while planning.
    Moving(Plan),
    Failed(NavFailure),
}

/// Shared navigation driver. Owns one planner request at a time, yields the legs to
/// reflexes (pauses the planner while pre-empted) and re-plans from the new position once
/// it gets them back.
#[derive(Debug, Default)]
pub struct Walker {
    target: Option<NavTarget>,
    paused: bool,
    failures: u32,
    planning: u32,
    scrape: Scrape,
    jolt: Jolt,
    /// Times the walk re-planned after a pre-emption.
    pub replans: u32,
    /// Scaffold blocks the paths of this walker placed, for the goal to clean up.
    pub placed: Vec<BlockPos>,
}

/// Where a target is now, if the snapshot still shows it.
pub fn target_pos(cx: &GoalCx, t: NavTarget) -> Option<Vec3> {
    match t {
        NavTarget::Block { pos, .. } => Some(pos.feet_centre()),
        NavTarget::Beside { block } => Some(block.feet_centre()),
        NavTarget::Sky { .. } => None,
        NavTarget::Entity { id, .. } => {
            let snap = cx.snap();
            snap.entity(id).map(|e| e.pos).or_else(|| {
                snap.companion
                    .as_ref()
                    .filter(|c| c.entity_id == Some(id))
                    .map(|c| c.pos)
            })
        }
    }
}

pub fn arrived(cx: &GoalCx, t: NavTarget) -> bool {
    let me = cx.me_pos();
    match t {
        NavTarget::Block { pos, radius } => {
            let c = pos.feet_centre() + Vec3::new(0.0, 0.5, 0.0);
            (me + Vec3::new(0.0, 0.5, 0.0)).dist(c) <= radius.max(0.5)
        }
        NavTarget::Entity { distance, .. } => {
            target_pos(cx, t).is_some_and(|p| p.dist(me) <= distance)
        }
        NavTarget::Beside { block } => {
            let n = me.block();
            let (dx, dz) = ((block.x - n.x).abs(), (block.z - n.z).abs());
            let level = block.y == n.y || block.y == n.y + 1;
            dx + dz == 1 && level && (me - n.feet_centre()).horizontal().len() < 0.4
        }
        // Only the navigator can tell open sky; it reports Arrived.
        NavTarget::Sky { .. } => false,
    }
}

impl Walker {
    pub fn walk(&mut self, cx: &mut GoalCx, target: NavTarget, note: &'static str) -> Walk {
        if arrived(cx, target) {
            self.stop(cx);
            self.failures = 0;
            return Walk::Arrived;
        }
        if cx.preempted {
            if self.target.is_some() && !self.paused {
                cx.nav.pause();
                self.paused = true;
            }
            return Walk::Moving(Plan::hold(note));
        }
        // Moved by something other than the legs: the route starts where the body was.
        if self.jolt.step(cx.snap().tick, cx.me_pos()) && self.target.take().is_some() {
            cx.nav.cancel();
        }
        if self.target != Some(target) || self.paused {
            if self.paused {
                self.replans += 1;
            } else {
                self.scrape.reset();
            }
            let snap = cx.ctx.snap;
            cx.nav.protect(&cx.ctx.knobs.protected);
            cx.nav.keep(cx.ledger.keep());
            cx.nav.request(target, snap);
            self.target = Some(target);
            self.paused = false;
        }
        let snap = cx.ctx.snap;
        let status = cx.nav.poll(snap);
        self.placed.extend(cx.nav.take_placed());
        match status {
            NavStatus::Moving(s) if s.work.is_some() => {
                self.planning = 0;
                self.scrape.reset();
                let w = s.work.expect("checked");
                match work::work_plan(cx, s, w, note) {
                    Ok(plan) => Walk::Moving(plan),
                    Err(f) => self.fail(cx, f),
                }
            }
            NavStatus::Moving(s) => {
                self.planning = 0;
                if self.scrape.step(cx.me_pos()) {
                    self.scrape.reset();
                    return self.fail(cx, NavFailure::Stalled);
                }
                let me = &cx.snap().me;
                let speed = if s.sprint && me.food > 6 {
                    Speed::Sprint
                } else {
                    Speed::Walk
                };
                let mut legs = LegsCmd::toward(heading_to(me.pos, s.waypoint), speed);
                legs.jump = s.jump;
                Walk::Moving(Plan::new(note, Commands::default().legs(legs)))
            }
            NavStatus::Arrived => {
                self.stop(cx);
                // Live, stalls counted across three separate escapes ended the third one.
                self.failures = 0;
                Walk::Arrived
            }
            NavStatus::Failed(f) => self.fail(cx, f),
            // A re-plan keeps the no-progress count: live, a body pulled back every tick
            // re-planned after each unstuck nudge (every 40 ticks) and never stalled.
            status => {
                self.planning += 1;
                if self.planning > PLANNING_LIMIT {
                    return self.fail(cx, NavFailure::Stalled);
                }
                if status == NavStatus::Idle {
                    self.target = None;
                }
                Walk::Moving(Plan::hold(note))
            }
        }
    }

    fn fail(&mut self, cx: &mut GoalCx, f: NavFailure) -> Walk {
        self.failures += 1;
        self.target = None;
        self.planning = 0;
        cx.nav.cancel();
        if self.failures >= MAX_NAV_FAILURES {
            self.failures = 0;
            Walk::Failed(f)
        } else {
            cx.blocked(format!("path problem ({f:?}), re-planning"));
            Walk::Moving(Plan::hold("goal_replan"))
        }
    }

    pub fn stop(&mut self, cx: &mut GoalCx) {
        if self.target.take().is_some() {
            cx.nav.cancel();
        }
        self.paused = false;
        self.planning = 0;
        self.scrape.reset();
        self.jolt.reset();
    }

    /// Forget the request without touching the planner (it was cancelled elsewhere).
    pub fn reset(&mut self) {
        self.target = None;
        self.paused = false;
        self.planning = 0;
        self.scrape.reset();
    }
}
