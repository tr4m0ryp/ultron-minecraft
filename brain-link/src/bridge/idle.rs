//! Keeps the thought layer from waiting while the body stands idle.
//!
//! Measured on the first real runs: most idle time came from `wait_event` calls made after
//! a goal had ended and before a new one was set, each blocking its full timeout. So the
//! first wait after the body goes idle returns at once (a nudge to set the next goal), and
//! any wait that starts with no goal running blocks at most `IDLE_WAIT_MAX_S`.

use std::time::Duration;

use crate::host::Host;

/// Longest block for a `wait_event` that starts while no goal runs.
pub const IDLE_WAIT_MAX_S: f64 = 45.0;

/// No goal is set, or the set goal has ended. A goal the goal layer has not reported on
/// yet (status for an older id, or no status at all) counts as running.
pub fn body_idle<H: Host>(host: &mut H) -> bool {
    let Some(id) = host.knobs().goal.as_ref().map(|g| g.id) else {
        return true;
    };
    host.goal_status()
        .is_some_and(|s| s.goal_id == id && s.state.is_final())
}

#[derive(Default)]
pub struct IdleState {
    /// No goal is running.
    pub idle: bool,
    /// The body went idle and no wait has returned since: the next idle wait returns at once.
    nudge: bool,
    /// `update` has run once, so `idle` describes the body and not the default.
    seen: bool,
}

impl IdleState {
    /// Records the body state after a tick; a running-to-idle change arms the nudge.
    pub fn update(&mut self, idle: bool) {
        if idle && !self.idle && self.seen {
            self.nudge = true;
        }
        self.seen = true;
        if !idle {
            self.nudge = false;
        }
        self.idle = idle;
    }

    /// How long a wait with no events pending may block: zero for the nudge, capped while
    /// idle, else as asked. Using the nudge spends it.
    pub fn budget(&mut self, asked: Duration) -> Duration {
        if !self.idle {
            return asked;
        }
        if std::mem::take(&mut self.nudge) {
            return Duration::ZERO;
        }
        asked.min(Duration::from_secs_f64(IDLE_WAIT_MAX_S))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ASK: Duration = Duration::from_secs(120);

    #[test]
    fn a_goal_ending_makes_the_next_wait_return_at_once_once() {
        let mut s = IdleState::default();
        s.update(false);
        assert_eq!(s.budget(ASK), ASK, "a running goal: block as asked");
        s.update(true);
        assert_eq!(s.budget(ASK), Duration::ZERO, "nudge after the goal ended");
        assert_eq!(
            s.budget(ASK),
            Duration::from_secs_f64(IDLE_WAIT_MAX_S),
            "a deliberate second wait blocks, capped"
        );
        s.update(true);
        assert_eq!(s.budget(ASK), Duration::from_secs_f64(IDLE_WAIT_MAX_S));
    }

    #[test]
    fn idle_from_the_start_is_capped_but_not_nudged() {
        let mut s = IdleState::default();
        s.update(true);
        assert_eq!(s.budget(ASK), Duration::from_secs_f64(IDLE_WAIT_MAX_S));
        assert_eq!(s.budget(Duration::from_secs(2)), Duration::from_secs(2));
    }

    #[test]
    fn a_new_goal_before_any_wait_cancels_the_nudge() {
        let mut s = IdleState::default();
        s.update(false);
        s.update(true);
        s.update(false);
        assert_eq!(s.budget(ASK), ASK);
        s.update(true);
        assert_eq!(s.budget(ASK), Duration::ZERO);
    }
}
