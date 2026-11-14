/// Where the active goal stands, published by the runner every tick it steps the goal and
/// read by the brain link for `observe`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoalState {
    Running,
    /// The goal is waiting on something it reported (no path, missing item, ...).
    Blocked,
    /// A reflex holds the body; the goal resumes when it lets go.
    Preempted,
    Done,
    Failed,
}

impl GoalState {
    pub fn label(self) -> &'static str {
        match self {
            GoalState::Running => "running",
            GoalState::Blocked => "blocked",
            GoalState::Preempted => "preempted",
            GoalState::Done => "done",
            GoalState::Failed => "failed",
        }
    }

    pub fn is_final(self) -> bool {
        matches!(self, GoalState::Done | GoalState::Failed)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GoalStatus {
    pub goal_id: u64,
    /// The goal's short name, e.g. `mine`.
    pub label: &'static str,
    pub state: GoalState,
    /// Units done out of the total, for counted goals (mine, collect, craft, give).
    pub progress: Option<(u32, u32)>,
    /// The latest progress note, blocked reason, or final outcome.
    pub note: Option<String>,
}

impl GoalStatus {
    /// One compact line, e.g. `#4 mine running 2/3: mined stone`.
    pub fn line(&self) -> String {
        let mut s = format!("#{} {} {}", self.goal_id, self.label, self.state.label());
        if let Some((d, t)) = self.progress {
            s.push_str(&format!(" {d}/{t}"));
        }
        if let Some(n) = &self.note {
            s.push_str(&format!(": {n}"));
        }
        s
    }
}
