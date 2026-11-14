//! Footwork against archers. A skeleton aims at where we are when it looses, with no lead,
//! so any sideways speed at that moment makes the arrow miss: zigzag in, or circle.

use crate::behaviours::common::clearance;
use crate::math::Vec3;
use crate::snapshot::Snapshot;

/// Ticks per zigzag leg.
pub const LEG_TICKS: u64 = 12;
/// Ground needed along a heading before taking it.
const ROOM: f64 = 1.5;

pub fn rotate(v: Vec3, deg: f64) -> Vec3 {
    let (s, c) = deg.to_radians().sin_cos();
    Vec3::new(v.x * c - v.z * s, 0.0, v.x * s + v.z * c)
}

/// Which side of the line to the archer we are moving to, and since when.
pub struct Footwork {
    side: f64,
    since: u64,
}

impl Default for Footwork {
    fn default() -> Self {
        Self {
            side: 1.0,
            since: 0,
        }
    }
}

impl Footwork {
    /// Heading `deg` off `toward`, on the current side. The side flips every `LEG_TICKS`,
    /// but never while `hold` (an arrow may be loosed soon: keep the sideways speed through
    /// the release), and at once when the side is walled. Both sides walled: `toward`.
    pub fn heading(&mut self, snap: &Snapshot, toward: Vec3, deg: f64, hold: bool) -> Vec3 {
        let now = snap.tick;
        if now >= self.since + LEG_TICKS && !hold {
            self.flip(now);
        }
        let open = |h: Vec3| clearance(&snap.blocks, snap.me.pos, h, ROOM).0 >= ROOM;
        let h = rotate(toward, self.side * deg);
        if open(h) {
            return h;
        }
        let other = rotate(toward, -self.side * deg);
        if open(other) {
            self.flip(now);
            return other;
        }
        toward
    }

    fn flip(&mut self, now: u64) {
        self.side = -self.side;
        self.since = now;
    }
}
