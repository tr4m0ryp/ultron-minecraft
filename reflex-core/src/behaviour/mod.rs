//! The behaviour contract: a trigger over the snapshot that yields a salience bid with
//! per-channel commands, and optionally a pre-action veto over the composed plan.

use std::cell::RefCell;

use crate::channels::Commands;
use crate::events::{Event, EventClass};
use crate::knobs::Knobs;
use crate::predict::Prediction;
use crate::snapshot::Snapshot;

/// Priority classes, design note section 4.2. P0 ignores brain inhibition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    P0 = 0,
    P1,
    P2,
    P3,
    P4,
    P5,
    P6,
}

impl Priority {
    /// class_base(P): one unit per class so the gain term never crosses a class boundary.
    pub fn base(self) -> f32 {
        (6 - self as u8) as f32
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bid {
    pub priority: Priority,
    /// Trigger strength u_b in 0..=1.
    pub urgency: f32,
    pub commands: Commands,
    /// Fallback needing fewer channels, tried when the full set is unavailable.
    pub degraded: Option<Commands>,
    /// Commitment lock in ticks for an atomic action this bid starts; only P0 breaks it.
    pub lock_ticks: u32,
    /// Short label for the upward channel, e.g. "lava_withdraw".
    pub note: &'static str,
}

impl Bid {
    pub fn new(priority: Priority, urgency: f32, note: &'static str, commands: Commands) -> Self {
        Self {
            priority,
            urgency: urgency.clamp(0.0, 1.0),
            commands,
            degraded: None,
            lock_ticks: 0,
            note,
        }
    }
    pub fn degraded(mut self, c: Commands) -> Self {
        self.degraded = Some(c);
        self
    }
    pub fn lock(mut self, ticks: u32) -> Self {
        self.lock_ticks = ticks;
        self
    }
}

/// A veto changed the plan before it reached the motors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Veto {
    pub priority: Priority,
    pub note: &'static str,
}

/// Read-only view handed to every behaviour each tick, plus an event sink.
pub struct Ctx<'a> {
    pub snap: &'a Snapshot,
    pub pred: &'a Prediction,
    pub knobs: &'a Knobs,
    pub bot_name: &'a str,
    events: RefCell<Vec<Event>>,
}

impl<'a> Ctx<'a> {
    pub fn new(
        snap: &'a Snapshot,
        pred: &'a Prediction,
        knobs: &'a Knobs,
        bot_name: &'a str,
    ) -> Self {
        Self {
            snap,
            pred,
            knobs,
            bot_name,
            events: RefCell::new(Vec::new()),
        }
    }

    pub fn emit(&self, ev: Event) {
        self.events.borrow_mut().push(ev);
    }

    /// Convenience for an event stamped with this tick and our position and health.
    pub fn report(&self, class: EventClass, kind: &str, subject: &str) -> Event {
        Event::new(self.snap.tick, class, kind, subject)
            .at(self.snap.me.pos)
            .hp(self.snap.me.health)
    }

    pub fn take_events(&self) -> Vec<Event> {
        std::mem::take(&mut self.events.borrow_mut())
    }
}

pub trait Behaviour {
    /// Catalog id, e.g. "R1". Inhibitions and per-behaviour gains address this id.
    fn id(&self) -> &'static str;

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid>;

    /// Pre-action veto over the composed plan (dig guard, edge guard, gaze filters).
    /// Returns `Some` only if it changed `plan`.
    fn veto(&mut self, _ctx: &Ctx, _plan: &mut Commands) -> Option<Veto> {
        None
    }

    /// Selection feedback: whether this behaviour's bid won any channel this tick.
    fn granted(&mut self, _won: bool) {}

    /// The flight-recorder probe this behaviour writes its state to, if any (the goal
    /// runner); the executor reads it into its ring every tick.
    fn probe(&self) -> Option<crate::trace::Probe> {
        None
    }
}
