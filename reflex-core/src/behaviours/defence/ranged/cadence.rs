//! Archer timing: when the next arrow can arrive, and whether a meal fits before it.

use crate::behaviours::common::sees;
use crate::math::Vec3;
use crate::snapshot::{Entity, Snapshot};

/// Ticks of draw before a skeleton looses its arrow.
pub const DRAW: u16 = 20;
/// Ticks from a shot to the next draw on Easy and Normal (20 on Hard).
pub const RELOAD: u16 = 40;
/// Skeleton arrow speed, blocks per tick.
pub const ARROW_SPEED: f64 = 1.6;
/// Archers engage within 15 blocks; one more for margin.
pub const RANGE: f64 = 16.0;

/// Ticks until `archer` can next loose: the rest of a draw in progress; after a seen shot,
/// the reload plus a full draw; otherwise a full draw (it may start drawing now).
pub fn ticks_to_shot(archer: &Entity) -> u16 {
    if archer.state.drawing_bow {
        return DRAW.saturating_sub(archer.state.draw_ticks);
    }
    match archer.state.since_shot {
        Some(s) => (RELOAD + DRAW).saturating_sub(s).max(DRAW),
        None => DRAW,
    }
}

/// The shot is seen a tick or two late; an arrow counts as in flight this much longer.
const SEEN_LATE: f64 = 3.0;

/// Ticks until an arrow from `archer` could reach `at`. Right after a shot the arrow may
/// still be in the air: live, lowering the shield the tick the draw ended let a 3-block
/// arrow land 2 ticks later behind it, 4 HP a time.
pub fn ticks_to_hit(archer: &Entity, at: Vec3) -> f64 {
    let flight = archer.pos.dist(at) / ARROW_SPEED;
    if let (false, Some(s)) = (archer.state.drawing_bow, archer.state.since_shot) {
        if s as f64 <= flight + SEEN_LATE {
            return (flight - s as f64).max(0.0);
        }
    }
    ticks_to_shot(archer) as f64 + flight
}

/// Archers within range that can see us, nearest first.
pub fn exposed_to(snap: &Snapshot) -> Vec<&Entity> {
    snap.nearby(RANGE, |e| e.kind.is_ranged())
        .into_iter()
        .filter(|e| sees(&snap.blocks, e.head(), snap.me.pos))
        .collect()
}

/// No archer that sees us can land an arrow within `ticks`: a 32-tick meal started now
/// finishes before any arrow arrives. Out of sight is safe; so is the reload after a shot.
pub fn safe_for(snap: &Snapshot, ticks: f64) -> bool {
    exposed_to(snap)
        .iter()
        .all(|e| ticks_to_hit(e, snap.me.pos) >= ticks)
}
