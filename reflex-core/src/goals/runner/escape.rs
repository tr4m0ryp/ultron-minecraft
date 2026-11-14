//! Escape: with no goal set, a body shut in (no open sky, and no walking route to it) for
//! long enough digs or pillars itself out without waiting for thought. Live, it once
//! spent 12 minutes in a 2-high cave pocket while every goal failed "unreachable".
//!
//! It never starts inside a protected region (the home the thought layer marked), and a
//! walking route to the sky, such as an open hut door, means "not enclosed": it does not
//! walk the body out of its shelter.

use crate::behaviour::{Bid, Ctx, Priority};
use crate::events::EventClass;
use crate::goals::ports::{NavStatus, NavTarget, Navigator, World};
use crate::goals::runner::Reporter;
use crate::goals::task::GoalCx;
use crate::goals::travel::{Walk, Walker};
use crate::math::BlockPos;

/// Ticks without sky (and without a goal) before the walking probe runs.
pub const ENCLOSED_TICKS: u32 = 200;
/// Ticks one escape may run before it gives up.
const ESCAPE_LIMIT: u32 = 2400;
/// Ticks between attempts after a probe found a way out on foot or an escape failed.
const COOLDOWN: u32 = 1200;
const SKY_CHECK_EVERY: u64 = 10;

#[derive(Debug)]
enum Phase {
    Watch,
    Probe,
    Dig(u32),
    Cooldown(u32),
}

pub struct Escape {
    phase: Phase,
    no_sky: u32,
    last_sky: Option<BlockPos>,
    walker: Walker,
    rep: Reporter,
}

impl Default for Escape {
    fn default() -> Self {
        Self {
            phase: Phase::Watch,
            no_sky: 0,
            last_sky: None,
            walker: Walker::default(),
            rep: Reporter::new(0, "escape"),
        }
    }
}

impl Escape {
    /// A goal took over: forget the count and any escape under way.
    pub fn reset(&mut self) {
        self.no_sky = 0;
        self.walker.reset();
        if matches!(self.phase, Phase::Probe | Phase::Dig(_)) {
            self.phase = Phase::Watch;
        }
    }

    pub fn running(&self) -> bool {
        matches!(self.phase, Phase::Probe | Phase::Dig(_))
    }

    fn say(ctx: &Ctx, text: &str) {
        ctx.emit(ctx.report(EventClass::Wake, "escape", "body").text(text.to_string()));
    }

    /// One idle tick. Returns the bid while an escape holds the body.
    pub fn tick(
        &mut self,
        ctx: &Ctx,
        nav: &mut dyn Navigator,
        world: &mut dyn World,
        preempted: bool,
    ) -> Option<Bid> {
        let feet = ctx.snap.me.pos.block();
        if ctx.snap.tick % SKY_CHECK_EVERY == 0 && !self.running() {
            if world.open_sky(feet) {
                self.last_sky = Some(feet);
                self.no_sky = 0;
            } else {
                self.no_sky += SKY_CHECK_EVERY as u32;
            }
        }
        let hint = self.last_sky.unwrap_or(feet.offset(0, 16, 0));
        match self.phase {
            Phase::Cooldown(0) => self.phase = Phase::Watch,
            Phase::Cooldown(n) => self.phase = Phase::Cooldown(n - 1),
            Phase::Watch => {
                let home = ctx.knobs.protected.iter().any(|r| r.contains(feet));
                if self.no_sky >= ENCLOSED_TICKS && !home {
                    nav.request(NavTarget::Sky { hint, dig: false }, ctx.snap);
                    self.phase = Phase::Probe;
                }
            }
            Phase::Probe => match nav.poll(ctx.snap) {
                NavStatus::Planning => return Some(Self::hold()),
                NavStatus::Failed(_) => {
                    nav.cancel();
                    Self::say(ctx, "shut in with no way out on foot; digging out");
                    self.phase = Phase::Dig(0);
                }
                _ => {
                    nav.cancel();
                    self.no_sky = 0;
                    self.phase = Phase::Cooldown(COOLDOWN);
                }
            },
            Phase::Dig(_) => {}
        }
        let Phase::Dig(t) = self.phase else {
            return None;
        };
        if t > ESCAPE_LIMIT {
            return self.give_up(ctx, nav, "ran out of time");
        }
        self.phase = Phase::Dig(t + u32::from(!preempted));
        let mut cx = GoalCx {
            ctx,
            nav: &mut *nav,
            world,
            rep: &mut self.rep,
            preempted,
        };
        let walk = self
            .walker
            .walk(&mut cx, NavTarget::Sky { hint, dig: true }, "escape");
        match walk {
            Walk::Moving(plan) => Some(Bid::new(Priority::P5, 0.3, plan.note, plan.commands)),
            Walk::Arrived => {
                Self::say(ctx, "out under open sky again");
                self.last_sky = Some(ctx.snap.me.pos.block());
                self.no_sky = 0;
                self.phase = Phase::Watch;
                None
            }
            Walk::Failed(f) => self.give_up(ctx, nav, &format!("{f:?}")),
        }
    }

    fn give_up(&mut self, ctx: &Ctx, nav: &mut dyn Navigator, why: &str) -> Option<Bid> {
        nav.cancel();
        self.walker.reset();
        Self::say(ctx, &format!("could not dig out ({why}); I need help or a goal"));
        self.phase = Phase::Cooldown(COOLDOWN * 2);
        None
    }

    fn hold() -> Bid {
        let plan = crate::goals::task::Plan::hold("escape");
        Bid::new(Priority::P5, 0.05, plan.note, plan.commands)
    }
}
