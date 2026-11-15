//! How long the edge guard has held a goal's legs, for the one wake that tells thought.

/// The guard holding a goal's legs this long wakes thought once per goal (3 s). Only
/// thought may inhibit the guard, so a held goal is a block the body cannot clear: in the
/// recorded play the two blocked goals thought did not wait on were both answered by
/// inhibiting R5 here.
pub const EDGE_HOLD_WAKE_TICKS: u32 = 60;
/// Holds further apart than this are counted afresh.
const HOLD_GAP_TICKS: u64 = 20;

#[derive(Default)]
pub struct Hold {
    goal: Option<u64>,
    ticks: u32,
    last: u64,
    woke: bool,
}

impl Hold {
    /// One held tick for `goal`; true on the tick the hold becomes worth a wake.
    pub fn observe(&mut self, goal: u64, now: u64) -> bool {
        if self.goal != Some(goal) {
            *self = Hold {
                goal: Some(goal),
                ..Hold::default()
            };
        } else if now.saturating_sub(self.last) > HOLD_GAP_TICKS {
            self.ticks = 0;
        }
        self.last = now;
        self.ticks += 1;
        if self.woke || self.ticks < EDGE_HOLD_WAKE_TICKS {
            return false;
        }
        self.woke = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wakes_once_per_goal_after_an_unbroken_hold() {
        let mut h = Hold::default();
        let n = u64::from(EDGE_HOLD_WAKE_TICKS);
        assert!((1..n).all(|t| !h.observe(7, t)));
        assert!(h.observe(7, n));
        assert!((n + 1..n + 200).all(|t| !h.observe(7, t)), "once per goal");
        assert!((0..n - 1).all(|t| !h.observe(8, 1000 + t)));
        assert!(h.observe(8, 1000 + n - 1), "a new goal counts afresh");
    }

    #[test]
    fn brief_holds_far_apart_never_add_up() {
        let mut h = Hold::default();
        for burst in 0..20u64 {
            for t in 0..10 {
                assert!(!h.observe(7, burst * 100 + t));
            }
        }
    }
}
