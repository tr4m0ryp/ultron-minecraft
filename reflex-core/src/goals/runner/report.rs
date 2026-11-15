use std::collections::HashMap;

use super::blocked::{Block, BlockWatch};
use super::status::{GoalState, GoalStatus};
use crate::behaviour::Ctx;
use crate::events::EventClass;

/// A blocked reason or an open-ended note is not repeated within this many ticks.
pub const REPEAT_GAP_TICKS: u64 = 200;

/// Per-goal status reporting without flooding: started once, progress at quarter
/// milestones, each blocked reason once until it clears (and not again within the gap),
/// done or failed once. A block wakes thought only when it is stuck or outlasts
/// `BLOCK_WAKE_TICKS` (see `blocked`); otherwise it is a digest line. Event text carries the goal id so identical outcomes of separate
/// goals are not mistaken for repeats.
#[derive(Debug)]
pub struct Reporter {
    goal_id: u64,
    label: &'static str,
    milestone: u32,
    blocked: Option<String>,
    blocked_this_tick: bool,
    watch: BlockWatch,
    last_sent: HashMap<String, u64>,
    progress: Option<(u32, u32)>,
    note: Option<String>,
    outcome: Option<(GoalState, String)>,
}

impl Reporter {
    pub fn new(goal_id: u64, label: &'static str) -> Self {
        Self {
            goal_id,
            label,
            milestone: 0,
            blocked: None,
            blocked_this_tick: false,
            watch: BlockWatch::default(),
            last_sent: HashMap::new(),
            progress: None,
            note: None,
            outcome: None,
        }
    }

    pub fn goal_id(&self) -> u64 {
        self.goal_id
    }

    pub fn label(&self) -> &'static str {
        self.label
    }

    fn send(&self, ctx: &Ctx, class: EventClass, kind: &str, text: &str) {
        let text = format!("#{} {}: {text}", self.goal_id, self.label);
        ctx.emit(ctx.report(class, kind, self.label).text(text));
    }

    pub fn started(&self, ctx: &Ctx, spec: &str) {
        self.send(ctx, EventClass::Digest, "goal_started", spec);
    }

    pub fn resumed(&self, ctx: &Ctx) {
        self.send(ctx, EventClass::Digest, "goal_resumed", "resumed");
    }

    pub fn suspended(&self, ctx: &Ctx) {
        self.send(
            ctx,
            EventClass::Digest,
            "goal_suspended",
            "replaced by a new goal",
        );
    }

    pub fn progress(&mut self, ctx: &Ctx, done: u32, total: u32, text: String) {
        if total > 0 {
            self.progress = Some((done.min(total), total));
        }
        if total == 0 || done >= total {
            return;
        }
        self.note = Some(text.clone());
        let m = done * 4 / total;
        if m > self.milestone {
            self.milestone = m;
            self.watch.progressed();
            self.send(ctx, EventClass::Digest, "goal_progress", &text);
        }
    }

    fn due(&mut self, key: &str, now: u64) -> bool {
        let due = self
            .last_sent
            .get(key)
            .is_none_or(|&t| now >= t + REPEAT_GAP_TICKS);
        if due {
            self.last_sent.insert(key.to_string(), now);
        }
        due
    }

    pub fn note(&mut self, ctx: &Ctx, text: String) {
        self.note = Some(text.clone());
        if self.due(&format!("note|{text}"), ctx.snap.tick) {
            self.send(ctx, EventClass::Digest, "goal_progress", &text);
        }
    }

    /// What the goal is doing now (the plan step under way), for the status only: no event.
    pub fn doing(&mut self, text: String) {
        self.note = Some(text);
    }

    pub fn blocked(&mut self, ctx: &Ctx, reason: String, how: Block) {
        self.blocked_this_tick = true;
        let now = ctx.snap.tick;
        if how == Block::Handled {
            if let Some(secs) = self.watch.observe(now) {
                let text = format!("{reason}; blocked for {secs} s");
                self.send(ctx, EventClass::Wake, "goal_blocked", &text);
            }
        }
        if self.blocked.as_deref() == Some(reason.as_str()) {
            return;
        }
        if self.due(&format!("blocked|{reason}"), now) {
            let class = match how {
                Block::Stuck => EventClass::Wake,
                Block::Handled => EventClass::Digest,
            };
            self.send(ctx, class, "goal_blocked", &reason);
        }
        self.blocked = Some(reason);
    }

    /// Clears the blocked state when the goal did not report it this tick.
    pub fn end_tick(&mut self) {
        if !self.blocked_this_tick {
            self.blocked = None;
        }
        self.blocked_this_tick = false;
    }

    pub fn is_blocked(&self) -> Option<&str> {
        self.blocked.as_deref()
    }

    /// Goal outcomes skip the 5 s wake gap: thought sets the next goal at once instead of
    /// standing idle while the result waits in the queue.
    pub fn done(&mut self, ctx: &Ctx, text: &str) {
        self.outcome = Some((GoalState::Done, text.to_string()));
        self.send_now(ctx, "goal_done", text);
    }

    pub fn failed(&mut self, ctx: &Ctx, reason: &str) {
        self.outcome = Some((GoalState::Failed, reason.to_string()));
        self.send_now(ctx, "goal_failed", reason);
    }

    fn send_now(&self, ctx: &Ctx, kind: &str, text: &str) {
        let text = format!("#{} {}: {text}", self.goal_id, self.label);
        ctx.emit(
            ctx.report(EventClass::Wake, kind, self.label)
                .text(text)
                .critical(),
        );
    }

    /// The goal's status for the brain; `preempted` says a reflex holds the body.
    pub fn status(&self, preempted: bool) -> GoalStatus {
        let (state, note) = match (&self.outcome, &self.blocked) {
            (Some((s, text)), _) => (*s, Some(text.clone())),
            (None, Some(reason)) => (GoalState::Blocked, Some(reason.clone())),
            (None, None) if preempted => (GoalState::Preempted, self.note.clone()),
            (None, None) => (GoalState::Running, self.note.clone()),
        };
        GoalStatus {
            goal_id: self.goal_id,
            label: self.label,
            state,
            progress: self.progress,
            note,
        }
    }
}
