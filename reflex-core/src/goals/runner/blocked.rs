//! When a blocked goal wakes thought. Over two days of recorded play thought answered a
//! `goal_blocked` wake by waiting in 46 of 52 cases: the body was already re-planning,
//! skipping the block or stepping aside, and a done or failed event followed by itself. So a
//! block the body works around goes into the digest. Thought is woken when the block is one
//! only it can resolve (`Block::Stuck`: nobody to follow, monsters round the bed) or when
//! a spell of blocks has lasted `BLOCK_WAKE_TICKS` with no progress in between.

/// A spell of blocks this long wakes thought once (10 s).
pub const BLOCK_WAKE_TICKS: u64 = 200;
/// Blocks further apart than this are separate spells (a navigator stall takes 100 ticks,
/// so a body re-planning in a loop stays in one spell).
pub const SPELL_GAP_TICKS: u64 = 120;

/// How a goal is blocked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Block {
    /// The body is working around it (a re-plan, a skipped block, a step aside).
    Handled,
    /// Nothing the body does will clear it; thought has to decide.
    Stuck,
}

#[derive(Clone, Copy, Debug)]
struct Spell {
    since: u64,
    last: u64,
    woke: bool,
}

/// Tracks the current spell of handled blocks for one goal.
#[derive(Clone, Copy, Debug, Default)]
pub struct BlockWatch {
    spell: Option<Spell>,
}

impl BlockWatch {
    /// Registers a handled block at `now`. Returns the seconds the spell has lasted when
    /// this report is the one that wakes thought (once per spell).
    pub fn observe(&mut self, now: u64) -> Option<u64> {
        let s = match &mut self.spell {
            Some(s) if now.saturating_sub(s.last) <= SPELL_GAP_TICKS => s,
            slot => slot.insert(Spell {
                since: now,
                last: now,
                woke: false,
            }),
        };
        s.last = now;
        let lasted = now - s.since;
        if s.woke || lasted < BLOCK_WAKE_TICKS {
            return None;
        }
        s.woke = true;
        Some(lasted / 20)
    }

    /// The goal moved on (a progress milestone): the spell is over.
    pub fn progressed(&mut self) {
        self.spell = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_spell_never_wakes_and_a_long_one_wakes_once() {
        let mut w = BlockWatch::default();
        assert!((0..BLOCK_WAKE_TICKS).all(|t| w.observe(1000 + t).is_none()));
        assert_eq!(w.observe(1000 + BLOCK_WAKE_TICKS), Some(10));
        assert!((1..400).all(|t| w.observe(1000 + BLOCK_WAKE_TICKS + t).is_none()));
    }

    #[test]
    fn re_plans_a_stall_apart_form_one_spell() {
        let mut w = BlockWatch::default();
        assert_eq!(w.observe(0), None);
        assert_eq!(w.observe(100), None);
        assert_eq!(w.observe(210), Some(10), "third stall, 10.5 s in");
    }

    #[test]
    fn a_gap_or_progress_starts_a_new_spell() {
        let mut w = BlockWatch::default();
        w.observe(0);
        assert_eq!(w.observe(SPELL_GAP_TICKS + 1), None, "new spell");
        assert_eq!(w.observe(SPELL_GAP_TICKS + 100), None);
        w.progressed();
        assert_eq!(w.observe(SPELL_GAP_TICKS + 201), None, "progress reset it");
    }
}
