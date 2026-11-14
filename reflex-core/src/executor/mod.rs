//! One tick of the reflex layer: sense, predict, bid, select, veto, compose, report.

mod armed;
mod report;

use std::collections::{HashMap, HashSet};

use crate::behaviour::{Behaviour, Ctx, Priority};
use crate::behaviours::default_catalog;
use crate::channels::Channel;
use crate::events::{Digest, Event, Upward};
use crate::knobs::Knobs;
use crate::predict::{Efference, Prediction};
use crate::selector::{compose, resolve_gaze, salience, Candidate, MotorOutput, Selector};
use crate::snapshot::Snapshot;

/// Goal loses the legs this long before the brain hears INTERRUPTED (design note 4.4, step 6).
pub const GOAL_INTERRUPT_TICKS: u64 = 200;
/// A veto is honoured unless gain * (1 - inhibition) falls below this.
const VETO_DRIVE_MIN: f32 = 0.5;

pub struct Executor {
    name: String,
    behaviours: Vec<Box<dyn Behaviour>>,
    pub knobs: Knobs,
    pub upward: Upward,
    selector: Selector,
    efference: Efference,
    /// Ids that won a channel (or vetoed) last tick, for onset-only reporting.
    active: HashMap<String, Priority>,
    goal_watch: GoalWatch,
    seen_kinds: HashSet<crate::snapshot::EntityKind>,
    /// Note of the bid that won the legs last tick, for the trace.
    legs_note: Option<&'static str>,
}

/// Tracks how long the goal has bid without owning the legs.
#[derive(Default)]
struct GoalWatch {
    lost_since: Option<u64>,
    sent: bool,
}

impl Executor {
    pub fn new(bot_name: &str) -> Self {
        Self::with_behaviours(bot_name, default_catalog())
    }

    pub fn with_behaviours(bot_name: &str, behaviours: Vec<Box<dyn Behaviour>>) -> Self {
        Self {
            name: bot_name.to_string(),
            behaviours,
            knobs: Knobs::new(),
            upward: Upward::default(),
            selector: Selector::default(),
            efference: Efference::default(),
            active: HashMap::new(),
            goal_watch: GoalWatch::default(),
            seen_kinds: HashSet::new(),
            legs_note: None,
        }
    }

    /// Id of the behaviour (or "ARM:<n>") that owned `c` on the last tick.
    pub fn owner(&self, c: Channel) -> Option<&str> {
        self.selector.holder(c)
    }

    /// Note of the bid that won the legs on the last tick (which tactic moved the body).
    pub fn legs_note(&self) -> Option<&'static str> {
        self.legs_note
    }

    pub fn take_wake(&mut self, now: u64) -> Vec<Event> {
        self.upward.take_wake(now)
    }

    pub fn take_digest(&mut self) -> Digest {
        self.upward.take_digest(self.knobs.report_tokens)
    }

    pub fn tick(&mut self, snap: &Snapshot) -> MotorOutput {
        let now = snap.tick;
        self.knobs.expire(now);
        let pred = Prediction::compute(snap, self.efference);
        let ctx = Ctx::new(snap, &pred, &self.knobs, &self.name);
        let pos = snap.me.pos;

        let mut cands: Vec<Candidate> = Vec::new();
        let mut notes: Vec<&'static str> = Vec::new();
        let mut owners_idx: Vec<Option<usize>> = Vec::new();
        for (k, b) in self.behaviours.iter_mut().enumerate() {
            let Some(bid) = b.evaluate(&ctx) else {
                continue;
            };
            let id = b.id();
            let inh = self.knobs.inhibition_for(id, pos, now);
            let s = salience(
                bid.priority,
                self.knobs.gains.behaviour_gain(id),
                bid.urgency,
                inh,
            );
            if s.overridden {
                ctx.emit(report::override_event(&ctx, id, bid.note));
            }
            if s.drive <= 0.0 {
                ctx.emit(report::suppressed(&ctx, id, bid.note));
                continue;
            }
            cands.push(Candidate {
                id: id.to_string(),
                priority: bid.priority,
                salience: s.value,
                commands: bid.commands,
                degraded: bid.degraded,
                lock_ticks: bid.lock_ticks,
            });
            notes.push(bid.note);
            owners_idx.push(Some(k));
        }
        let armed = armed::bids(&ctx);
        for (arm_id, bid) in &armed {
            let s = salience(bid.priority, 1.0, bid.urgency, 0.0);
            cands.push(Candidate {
                id: format!("ARM:{arm_id}"),
                priority: bid.priority,
                salience: s.value,
                commands: bid.commands.clone(),
                degraded: None,
                lock_ticks: 0,
            });
            notes.push(bid.note);
            owners_idx.push(None);
        }

        let alloc = self.selector.select(&cands, now);
        self.legs_note = alloc.owners.get(&Channel::Legs).map(|&i| notes[i]);
        let mut won = vec![false; self.behaviours.len()];
        for &w in &alloc.winners {
            if let Some(k) = owners_idx[w] {
                won[k] = true;
            }
        }
        for (k, b) in self.behaviours.iter_mut().enumerate() {
            b.granted(won[k]);
        }

        let mut plan = alloc.plan.clone();
        resolve_gaze(snap, &mut plan);
        let mut now_active: HashMap<String, Priority> = HashMap::new();
        for b in self.behaviours.iter_mut() {
            let mut trial = plan.clone();
            let Some(v) = b.veto(&ctx, &mut trial) else {
                continue;
            };
            let id = b.id();
            let inh = self.knobs.inhibition_for(id, pos, now);
            let drive = self.knobs.gains.behaviour_gain(id) * (1.0 - inh);
            if v.priority == Priority::P0 || drive >= VETO_DRIVE_MIN {
                plan = trial;
                if v.priority == Priority::P0 && inh > 0.0 {
                    ctx.emit(report::override_event(&ctx, id, v.note));
                }
                if !self.active.contains_key(id) {
                    ctx.emit(report::fired(&ctx, id, v.note, v.priority, &self.knobs));
                }
                now_active.insert(id.to_string(), v.priority);
            } else {
                ctx.emit(report::suppressed(&ctx, id, v.note));
            }
        }
        let out = compose(snap, &plan);

        for &w in &alloc.winners {
            let c = &cands[w];
            if !self.active.contains_key(&c.id) {
                if let Some(arm_id) = c.id.strip_prefix("ARM:") {
                    ctx.emit(report::armed_fired(&ctx, arm_id, notes[w]));
                } else {
                    ctx.emit(report::fired(
                        &ctx,
                        &c.id,
                        notes[w],
                        c.priority,
                        &self.knobs,
                    ));
                }
            }
            now_active.insert(c.id.clone(), c.priority);
        }
        self.goal_watch.update(&ctx, &cands, &alloc.owners);
        report::first_sightings(&ctx, &mut self.seen_kinds);
        let events = ctx.take_events();
        drop(ctx);

        armed::retire_fired(&mut self.knobs, &cands, &alloc.winners);
        for e in events {
            self.upward.push(e);
        }
        self.active = now_active;
        self.efference = Efference {
            legs: plan.legs,
            from: pos,
        };
        out
    }
}

impl GoalWatch {
    fn update(&mut self, ctx: &Ctx, cands: &[Candidate], owners: &HashMap<Channel, usize>) {
        let goal_bidding = cands.iter().any(|c| c.id == crate::behaviours::GOAL_ID);
        let goal_has_legs = owners
            .get(&Channel::Legs)
            .is_some_and(|&i| cands[i].id == crate::behaviours::GOAL_ID);
        if !goal_bidding || goal_has_legs {
            self.lost_since = None;
            self.sent = false;
            return;
        }
        let since = *self.lost_since.get_or_insert(ctx.snap.tick);
        if !self.sent && ctx.snap.tick >= since + GOAL_INTERRUPT_TICKS {
            self.sent = true;
            let holder = owners
                .get(&Channel::Legs)
                .map(|&i| cands[i].id.as_str())
                .unwrap_or("none");
            ctx.emit(report::goal_interrupted(ctx, holder));
        }
    }
}
