//! Closes a container menu that stays open with nobody using it. Live: a place goal clicked
//! a crafting table, its menu stayed open, and every later 2x2 craft, equip swap and toss
//! was skipped or failed with "another container is open" until the session ended.

use azalea::Client;

/// The crafter keeps a table open for at most about 50 ticks (open wait plus settle), so a
/// menu open longer than this belongs to no job.
pub const STRAY_TICKS: u32 = 100;

#[derive(Default)]
pub struct MenuGuard {
    open_ticks: u32,
}

impl MenuGuard {
    /// Counts consecutive ticks with a container open; true when it should be closed now.
    pub fn observe(&mut self, open: bool) -> bool {
        if !open {
            self.open_ticks = 0;
            return false;
        }
        self.open_ticks += 1;
        if self.open_ticks > STRAY_TICKS {
            self.open_ticks = 0;
            return true;
        }
        false
    }
}

pub fn apply(bot: &Client, guard: &mut MenuGuard) {
    let Ok(handle) = bot.get_inventory() else {
        return;
    };
    if guard.observe(handle.id() != 0) {
        eprintln!("closing a container menu left open for {STRAY_TICKS} ticks");
        handle.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closes_only_after_a_long_unbroken_stretch() {
        let mut g = MenuGuard::default();
        assert!((0..STRAY_TICKS).all(|_| !g.observe(true)));
        assert!(g.observe(true), "stray menu closed");
        assert!(!g.observe(true), "counter restarts after closing");
        let mut g = MenuGuard::default();
        for _ in 0..60 {
            assert!(!g.observe(true));
        }
        assert!(!g.observe(false));
        assert!(
            (0..STRAY_TICKS).all(|_| !g.observe(true)),
            "a crafter's short open resets"
        );
    }
}
