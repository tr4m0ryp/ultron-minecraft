use crate::math::Vec3;

/// Blocks along the ground in one tick that no walk, sprint or knockback covers.
const JOLT_XZ: f64 = 2.0;
/// Blocks up or down in one tick beyond the fastest fall (3.92).
const JOLT_Y: f64 = 4.5;

/// Catches a body moved by something other than its legs between two ticks in a row (a
/// teleport, a respawn). The route under way then starts somewhere the body no longer is.
/// Live, a body teleported back into its hut while it was digging out followed the old
/// route into a wall for 100 ticks, three times, and the escape gave up.
#[derive(Debug, Default)]
pub struct Jolt {
    last: Option<(u64, Vec3)>,
}

impl Jolt {
    pub fn reset(&mut self) {
        self.last = None;
    }

    /// One tick at `pos`; true when the body jumped since the tick before.
    pub fn step(&mut self, tick: u64, pos: Vec3) -> bool {
        let jolted = self.last.is_some_and(|(t, p)| {
            tick == t + 1
                && ((pos - p).horizontal().len() > JOLT_XZ || (pos.y - p.y).abs() > JOLT_Y)
        });
        self.last = Some((tick, pos));
        jolted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_teleport_is_a_jolt_and_a_sprint_or_a_fall_is_not() {
        let mut j = Jolt::default();
        assert!(!j.step(10, Vec3::new(0.0, 64.0, 0.0)), "first tick");
        assert!(!j.step(11, Vec3::new(0.4, 64.0, 0.3)), "sprinting");
        assert!(!j.step(12, Vec3::new(0.8, 60.1, 0.6)), "falling fast");
        assert!(j.step(13, Vec3::new(3.5, 62.0, 3.5)), "teleported");
        // A gap in the ticks (the walk was not asked) proves nothing.
        assert!(!j.step(40, Vec3::new(90.0, 62.0, 3.5)));
    }
}
