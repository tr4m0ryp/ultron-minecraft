//! Finishes or fails the active goal the way the goal layer reports it: counted goals take
//! a few seconds per unit and end with `goal_done`; a matching fail rule ends them with
//! `goal_failed` and a reason; go_to and come_here end on arrival.

use reflex_core::events::EventClass;
use reflex_core::goals::{GoalState, GoalStatus};

use super::note;
use crate::fake::FakeWorld;

const TICKS_PER_UNIT: u64 = 60;
const FAIL_AFTER: u64 = 40;
const ARRIVED: f64 = 1.0;

pub struct FailRule {
    goal: String,
    kind: String,
    times: u32,
    reason: String,
}

impl FailRule {
    /// `<goal> <kind|*> <times> <reason...>`
    pub fn parse(s: &str) -> Option<Self> {
        let mut it = s.splitn(4, ' ');
        let goal = it.next()?.to_string();
        let kind = it.next()?.to_string();
        let times = it.next()?.parse().ok()?;
        let reason = it.next().unwrap_or("cannot do that here").to_string();
        Some(Self { goal, kind, times, reason })
    }

    fn matches(&self, label: &str, spec: &str) -> bool {
        self.times > 0
            && self.goal == label
            && (self.kind == "*" || spec.split_whitespace().any(|w| w == self.kind))
    }
}

enum Plan {
    Units { total: u32 },
    Fail(String),
    Arrive,
    Hold,
}

struct Running {
    id: u64,
    spec: String,
    label: &'static str,
    started: u64,
    plan: Plan,
}

#[derive(Default)]
pub struct GoalSim {
    pub rules: Vec<FailRule>,
    run: Option<Running>,
}

impl GoalSim {
    pub fn tick(&mut self, world: &mut FakeWorld) {
        let Some(goal) = world.exec.knobs.goal.clone() else {
            self.run = None;
            return;
        };
        if self.run.as_ref().map(|r| r.id) != Some(goal.id) {
            note(&format!("goal set: #{} {}", goal.id, goal.spec));
            self.run = Some(self.plan(goal.id, &goal.spec, world.snap.tick));
        }
        let now = world.snap.tick;
        let r = self.run.as_ref().expect("set above");
        let age = now.saturating_sub(r.started);
        let status = |state, progress, note: &str| GoalStatus {
            goal_id: r.id,
            label: r.label,
            state,
            progress,
            note: Some(note.to_string()),
        };
        let outcome = match &r.plan {
            Plan::Hold => None,
            Plan::Fail(reason) if age >= FAIL_AFTER => Some((false, reason.clone())),
            Plan::Fail(_) => {
                world.exec.status = Some(status(GoalState::Running, None, "looking"));
                None
            }
            Plan::Arrive => {
                let t = goal.target.unwrap_or(world.snap.me.pos);
                let d = (t - world.snap.me.pos).horizontal().len();
                (d <= ARRIVED).then(|| (true, format!("#{} {}: arrived", r.id, r.spec)))
            }
            Plan::Units { total } => {
                let done = ((age / TICKS_PER_UNIT) as u32).min(*total);
                let line = format!("{} {done}/{total}", r.label);
                world.exec.status = Some(status(GoalState::Running, Some((done, *total)), &line));
                (done >= *total).then(|| (true, format!("#{} {}: done", r.id, r.spec)))
            }
        };
        let Some((ok, text)) = outcome else { return };
        let (state, kind) = match ok {
            true => (GoalState::Done, "goal_done"),
            false => (GoalState::Failed, "goal_failed"),
        };
        world.exec.status = Some(status(state, None, &text));
        let text = match ok {
            true => text,
            false => format!("#{} {}: {text}", r.id, r.spec),
        };
        world.push(EventClass::Wake, kind, r.label, &text);
        note(&format!("{kind}: {text}"));
        world.exec.knobs.end_goal();
        self.run = None;
    }

    fn plan(&mut self, id: u64, spec: &str, now: u64) -> Running {
        let label = label_of(spec);
        let rule = self.rules.iter_mut().find(|r| r.matches(label, spec));
        let plan = if let Some(rule) = rule {
            rule.times -= 1;
            Plan::Fail(rule.reason.clone())
        } else {
            match label {
                "go_to" | "come_here" => Plan::Arrive,
                "follow" | "guard" | "stay" | "other" => Plan::Hold,
                _ => Plan::Units {
                    total: count_of(spec).clamp(1, 8),
                },
            }
        };
        Running {
            id,
            spec: spec.to_string(),
            label,
            started: now,
            plan,
        }
    }
}

fn label_of(spec: &str) -> &'static str {
    const LABELS: [&str; 13] = [
        "follow", "go_to", "come_here", "stay", "mine", "collect", "attack", "guard", "craft",
        "give", "place", "sleep_in_bed", "eat_now",
    ];
    let first = spec.split_whitespace().next().unwrap_or("");
    LABELS.into_iter().find(|l| *l == first).unwrap_or("other")
}

/// The last integer word of a counted spec (`mine iron_ore 3`, `give companion bread 2`).
fn count_of(spec: &str) -> u32 {
    match label_of(spec) {
        "mine" | "collect" | "craft" | "give" => spec
            .split_whitespace()
            .last()
            .and_then(|w| w.parse().ok())
            .unwrap_or(1),
        _ => 1,
    }
}
