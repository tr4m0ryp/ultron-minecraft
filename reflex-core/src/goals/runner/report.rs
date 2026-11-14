use std::collections::HashMap;

use crate::behaviour::Ctx;
use crate::events::EventClass;

/// A blocked reason or an open-ended note is not repeated within this many ticks.
pub const REPEAT_GAP_TICKS: u64 = 200;

/// Per-goal status reporting without flooding: started once, progress at quarter
/// milestones, each blocked reason once until it clears (and not again within the gap),
/// done or failed once. Event text carries the goal id so identical outcomes of separate
/// goals are not mistaken for repeats.
#[derive(Debug)]
pub struct Reporter {
    goal_id: u64,
    label: &'static str,
    milestone: u32,
    blocked: Option<String>,
    blocked_this_tick: bool,
    last_sent: HashMap<String, u64>,
}

impl Reporter {
    pub fn new(goal_id: u64, label: &'static str) -> Self {
        Self {
            goal_id,
            label,
            milestone: 0,
            blocked: None,
            blocked_this_tick: false,
            last_sent: HashMap::new(),
        }
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
        if total == 0 || done >= total {
            return;
        }
        let m = done * 4 / total;
        if m > self.milestone {
            self.milestone = m;
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
        if self.due(&format!("note|{text}"), ctx.snap.tick) {
            self.send(ctx, EventClass::Digest, "goal_progress", &text);
        }
    }

    pub fn blocked(&mut self, ctx: &Ctx, reason: String) {
        self.blocked_this_tick = true;
        if self.blocked.as_deref() == Some(reason.as_str()) {
            return;
        }
        let key = format!("blocked|{reason}");
        if self.due(&key, ctx.snap.tick) {
            self.send(ctx, EventClass::Wake, "goal_blocked", &reason);
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

    pub fn done(&self, ctx: &Ctx, text: &str) {
        self.send(ctx, EventClass::Wake, "goal_done", text);
    }

    pub fn failed(&self, ctx: &Ctx, reason: &str) {
        self.send(ctx, EventClass::Wake, "goal_failed", reason);
    }
}
