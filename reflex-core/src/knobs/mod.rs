//! Downward channel: the knobs the brain turns. Everything persists until replaced or
//! expired.

mod arm;
mod gains;
mod goal;
mod inhibit;
mod mode;

pub use arm::{ArmedResponse, Response, Trigger};
pub use gains::{GainName, Gains, MLG_LEAD};
pub use goal::Goal;
pub use inhibit::{Inhibition, Scope};
pub use mode::Mode;

use std::collections::VecDeque;

use crate::goals::travel::route::Region;
use crate::math::Vec3;

pub const GOAL_QUEUE_MAX: usize = 3;

#[derive(Clone, Debug, Default)]
pub struct Knobs {
    pub gains: Gains,
    pub mode: Mode,
    pub goal: Option<Goal>,
    pub goal_queue: VecDeque<Goal>,
    pub inhibitions: Vec<Inhibition>,
    pub armed: Vec<ArmedResponse>,
    /// DIGEST token budget (the report knob).
    pub report_tokens: usize,
    /// Builds the body must never break, marked by the thought layer.
    pub protected: Vec<Region>,
    /// Cells of blocks the player placed (and the margin around the companion's), refreshed
    /// by the adapter each tick from the goal layer's ledger; reflexes never dig them.
    pub kept: crate::goals::Keep,
    next_id: u64,
}

impl Knobs {
    pub fn new() -> Self {
        Self {
            report_tokens: crate::events::DEFAULT_DIGEST_TOKENS,
            ..Default::default()
        }
    }

    fn fresh_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.gains = mode.preset();
    }

    /// Marks a region the body must never break; returns how many are marked.
    pub fn protect(&mut self, r: Region) -> usize {
        if !self.protected.contains(&r) {
            self.protected.push(r);
        }
        self.protected.len()
    }

    /// Lifts protection from every region containing `p`; returns how many were lifted.
    pub fn unprotect(&mut self, p: crate::math::BlockPos) -> usize {
        let before = self.protected.len();
        self.protected.retain(|r| !r.contains(p));
        before - self.protected.len()
    }

    pub fn set_gain(&mut self, name: GainName, value: f32) {
        self.gains.set(name, value);
    }

    /// Sets the active goal, pushing any current goal onto the queue (capped at three).
    pub fn set_goal(
        &mut self,
        spec: impl Into<String>,
        priority: f32,
        target: Option<Vec3>,
        solo: bool,
    ) -> u64 {
        let id = self.fresh_id();
        let goal = Goal {
            id,
            spec: spec.into(),
            priority: priority.clamp(0.0, 1.0),
            target,
            solo,
        };
        if let Some(old) = self.goal.replace(goal) {
            self.goal_queue.push_front(old);
            self.goal_queue.truncate(GOAL_QUEUE_MAX);
        }
        id
    }

    /// Ends the active goal, drops inhibitions scoped to it, and promotes the next queued goal.
    pub fn end_goal(&mut self) -> Option<Goal> {
        let done = self.goal.take();
        if let Some(g) = &done {
            self.inhibitions
                .retain(|i| i.scope != Scope::UntilGoalEnds(g.id));
        }
        self.goal = self.goal_queue.pop_front();
        done
    }

    pub fn inhibit(
        &mut self,
        behaviour: &str,
        scope: Scope,
        strength: f32,
        ttl: Option<u64>,
        now: u64,
    ) -> u64 {
        let id = self.fresh_id();
        self.inhibitions.push(Inhibition {
            id,
            behaviour: behaviour.to_string(),
            scope,
            strength: strength.clamp(0.0, 1.0),
            expires_at: ttl.map(|t| now + t),
        });
        id
    }

    pub fn arm(
        &mut self,
        trigger: Trigger,
        response: Response,
        ttl: Option<u64>,
        once: bool,
        now: u64,
    ) -> u64 {
        let id = self.fresh_id();
        self.armed.push(ArmedResponse {
            id,
            trigger,
            response,
            expires_at: ttl.map(|t| now + t),
            once,
        });
        id
    }

    /// Strongest active inhibition on a behaviour at this place and time, in 0..=1.
    pub fn inhibition_for(&self, behaviour: &str, pos: Vec3, now: u64) -> f32 {
        let goal = self.goal.as_ref().map(|g| g.id);
        self.inhibitions
            .iter()
            .filter(|i| i.behaviour == behaviour && i.active(pos, now, goal))
            .map(|i| i.strength)
            .fold(0.0, f32::max)
    }

    /// Removes expired inhibitions and armed responses; returns how many inhibitions expired.
    pub fn expire(&mut self, now: u64) -> usize {
        let before = self.inhibitions.len();
        self.inhibitions.retain(|i| !i.expired(now));
        self.armed.retain(|a| a.expires_at.is_none_or(|t| now < t));
        before - self.inhibitions.len()
    }
}
