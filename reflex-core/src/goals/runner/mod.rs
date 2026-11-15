//! The goal behaviour: one P5 bidder (id `GOAL`) that parses the brain's goal spec into a
//! resumable state machine, steps it each tick, and reports its status upward.

mod escape;
mod handle;
mod probe;
mod report;
mod status;

pub use escape::ENCLOSED_TICKS;
pub use handle::GoalHandle;
pub use report::{Reporter, REPEAT_GAP_TICKS};
pub use status::{GoalState, GoalStatus};

use super::ports::{Navigator, World};
use super::spec;
use super::task::{GoalCx, GoalTask, Intent, Step};
use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::behaviours::GOAL_ID;
use crate::knobs::{Goal, GOAL_QUEUE_MAX};
use crate::trace::{GoalTick, NavCode, Probe};
use probe::{Seen, SeenCell, StallWatch, TracedNav};

struct Active {
    goal_id: u64,
    task: Option<Box<dyn GoalTask>>,
    rep: Reporter,
}

pub struct GoalRunner {
    nav: Box<dyn Navigator>,
    world: Box<dyn World>,
    handle: GoalHandle,
    active: Option<Active>,
    /// Goals replaced by a newer one, kept so they resume where they were when the brain's
    /// queue promotes them again.
    suspended: Vec<Active>,
    bid_last: bool,
    preempted: bool,
    intent: Option<Intent>,
    escape: escape::Escape,
    seen: SeenCell,
    probe: Probe,
    watch: StallWatch,
}

impl GoalRunner {
    pub fn new(nav: Box<dyn Navigator>, world: Box<dyn World>, handle: GoalHandle) -> Self {
        let seen = SeenCell::default();
        let probe = handle.probe();
        Self {
            nav: Box::new(TracedNav::new(nav, seen.clone())),
            world,
            handle,
            active: None,
            suspended: Vec::new(),
            bid_last: false,
            preempted: false,
            intent: None,
            escape: escape::Escape::default(),
            seen,
            probe,
            watch: StallWatch::default(),
        }
    }

    fn retire_active(&mut self, ctx: &Ctx, keep: bool) {
        let Some(mut a) = self.active.take() else {
            return;
        };
        if let Some(t) = a.task.as_mut() {
            t.stop(self.nav.as_mut());
            if keep {
                a.rep.suspended(ctx);
                self.suspended.push(a);
                if self.suspended.len() > GOAL_QUEUE_MAX {
                    self.suspended.remove(0);
                }
            }
        }
        self.nav.cancel();
        self.preempted = false;
        self.intent = None;
    }

    /// Makes `goal` the active one: resumes it if suspended, else parses its spec.
    fn activate(&mut self, ctx: &Ctx, goal: &Goal) {
        if let Some(i) = self.suspended.iter().position(|a| a.goal_id == goal.id) {
            let a = self.suspended.remove(i);
            a.rep.resumed(ctx);
            self.active = Some(a);
            return;
        }
        let parsed = spec::parse(&goal.spec, goal.target);
        let label = parsed.as_ref().map_or("goal", |s| s.label());
        let mut rep = Reporter::new(goal.id, label);
        let task = match parsed {
            Ok(s) => {
                rep.started(ctx, &goal.spec);
                Some(super::build(&s))
            }
            Err(e) => {
                rep.failed(ctx, &format!("cannot parse goal: {e}"));
                self.handle.finish(goal.id);
                self.handle.publish(rep.status(false));
                None
            }
        };
        self.active = Some(Active {
            goal_id: goal.id,
            task,
            rep,
        });
    }

    fn finish(&mut self, ctx: &Ctx, step: &Step) {
        let Some(a) = self.active.as_mut() else {
            return;
        };
        if let Step::Failed(reason) = step {
            let why = format!("#{} {} failed: {reason}", a.goal_id, a.rep.label());
            request_dump(&self.probe, &mut self.watch, ctx.snap.tick, why);
        }
        match step {
            Step::Done(text) => a.rep.done(ctx, text),
            Step::Failed(reason) => a.rep.failed(ctx, reason),
            _ => return,
        }
        if let Some(mut t) = a.task.take() {
            t.stop(self.nav.as_mut());
        }
        self.nav.cancel();
        self.handle.finish(a.goal_id);
        self.handle.publish(a.rep.status(false));
    }
}

/// Asks the executor to print its ring, at most once per `DUMP_GAP_TICKS`.
fn request_dump(probe: &Probe, watch: &mut StallWatch, tick: u64, reason: String) {
    if watch.allow(tick) {
        probe.request_dump(reason);
    }
}

impl GoalRunner {
    /// Publishes this tick's goal state to the flight recorder and raises a dump on a
    /// navigator stall or a moving body that went nowhere.
    fn record(&mut self, ctx: &Ctx, bid: Option<&Bid>) {
        let seen = self.seen.get();
        let (goal_id, label, blocked) = self.active.as_ref().map_or((0, "", false), |a| {
            (a.goal_id, a.rep.label(), a.rep.is_blocked().is_some())
        });
        let g = GoalTick {
            goal_id,
            label,
            phase: bid.map_or("", |b| b.note),
            moving: bid.is_some_and(|b| b.commands.legs.as_ref().is_some_and(probe::asks_to_move)),
            nav: seen.nav,
            flags: seen.flags,
            wp: seen.wp,
            preempted: self.preempted,
            blocked,
        };
        self.probe.set(g);
        let tick = ctx.snap.tick;
        let why = if seen.nav == NavCode::Stalled {
            Some(format!("#{goal_id} {label} navigator Stalled"))
        } else {
            self.watch.observe(&g, ctx.snap.me.pos)
        };
        if let Some(why) = why {
            request_dump(&self.probe, &mut self.watch, tick, why);
        }
    }

    fn eval(&mut self, ctx: &Ctx) -> Option<Bid> {
        self.bid_last = false;
        let Some(goal) = ctx.knobs.goal.as_ref() else {
            self.retire_active(ctx, false);
            self.suspended.clear();
            let preempted = self.preempted;
            let bid = self.escape.tick(
                ctx,
                self.nav.as_mut(),
                self.world.as_mut(),
                self.handle.ledger(),
                preempted,
            );
            self.bid_last = bid.is_some();
            return bid;
        };
        if self.escape.running() {
            self.nav.cancel();
        }
        self.escape.reset();
        if self.active.as_ref().map(|a| a.goal_id) != Some(goal.id) {
            let queued = |id: u64| ctx.knobs.goal_queue.iter().any(|g| g.id == id);
            let keep = self.active.as_ref().is_some_and(|a| queued(a.goal_id));
            self.retire_active(ctx, keep);
            self.suspended
                .retain(|a| queued(a.goal_id) || a.goal_id == goal.id);
            self.activate(ctx, goal);
        }
        let a = self.active.as_mut()?;
        let task = a.task.as_mut()?;
        let mut cx = GoalCx {
            ctx,
            nav: self.nav.as_mut(),
            world: self.world.as_mut(),
            rep: &mut a.rep,
            ledger: self.handle.ledger(),
            preempted: self.preempted,
        };
        let step = task.step(&mut cx);
        a.rep.end_tick();
        self.handle.publish(a.rep.status(self.preempted));
        match step {
            Step::Bid(plan) => {
                self.intent = plan.intent;
                self.bid_last = true;
                let mut bid = Bid::new(
                    Priority::P5,
                    goal.priority.max(0.05),
                    plan.note,
                    plan.commands,
                )
                .lock(plan.lock);
                if let Some(d) = plan.degraded {
                    bid = bid.degraded(d);
                }
                Some(bid)
            }
            Step::Idle => None,
            done => {
                self.finish(ctx, &done);
                None
            }
        }
    }
}

impl Behaviour for GoalRunner {
    fn id(&self) -> &'static str {
        GOAL_ID
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        self.seen.set(Seen::default());
        let bid = self.eval(ctx);
        self.record(ctx, bid.as_ref());
        bid
    }

    fn probe(&self) -> Option<Probe> {
        Some(self.probe.clone())
    }

    fn granted(&mut self, won: bool) {
        self.preempted = self.bid_last && !won;
        let pre = self.preempted;
        self.probe.update(|g| g.preempted = pre);
        let intent = self.intent.take();
        if !won {
            return;
        }
        let Some(intent) = intent else {
            return;
        };
        match &intent {
            Intent::Craft {
                recipe,
                crafts,
                table,
            } => self.world.start_craft(recipe, *crafts, *table),
            Intent::Furnace { pos, job } => self.world.start_furnace(*pos, job),
            Intent::Toss { item, count } => {
                self.world.toss(item, *count);
            }
        }
        if let Some(t) = self.active.as_mut().and_then(|a| a.task.as_mut()) {
            t.committed();
        }
    }
}
