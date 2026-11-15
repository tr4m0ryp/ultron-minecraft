use crate::math::Vec3;

/// Ticks of walking orders without getting anywhere (10 s) before the walk counts as stalled.
pub(super) const SCRAPE_TICKS: u32 = 200;
/// Blocks the body must move to count as progress.
const PROGRESS_BLOCKS: f64 = 0.5;

/// Catches a body that keeps being told to walk but stays put, across re-plans. The path
/// follower's own stall count restarts whenever a reflex takes the legs for a tick (the
/// unstuck reflex does, every few ticks, against a wall) and the walker re-plans, so on its
/// own it never fires; this one only counts walking ticks and resets on real displacement.
#[derive(Debug, Default)]
pub struct Scrape {
    anchor: Option<Vec3>,
    still: u32,
}

impl Scrape {
    pub fn reset(&mut self) {
        self.anchor = None;
        self.still = 0;
    }

    /// One walking tick at `pos`; true once the body has gone nowhere for `SCRAPE_TICKS`.
    pub fn step(&mut self, pos: Vec3) -> bool {
        let a = *self.anchor.get_or_insert(pos);
        if pos.dist(a) >= PROGRESS_BLOCKS {
            self.anchor = Some(pos);
            self.still = 0;
            return false;
        }
        self.still += 1;
        self.still >= SCRAPE_TICKS
    }
}
