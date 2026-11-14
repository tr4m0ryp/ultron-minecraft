//! The hand work of one hop: break what blocks it (top-down), place the block it stands
//! on, and only then let the legs move.

use crate::goals::ports::{NavFailure, Steer, Work};
use crate::goals::travel::route::{Cell, Hop, HopKind, Terrain};
use crate::math::{BlockPos, Vec3};
use crate::snapshot::Snapshot;

/// Ticks one block may take to break before the hop counts as stuck (a veto, a wrong
/// tool, a block the server will not let go).
pub const DIG_LIMIT: u32 = 600;
/// Ticks one placement may take (several attempts) before the hop counts as stuck.
const PLACE_LIMIT: u32 = 80;
/// Ticks between placement clicks, so the server's answer arrives before a retry.
const CLICK_GAP: u32 = 4;
/// Feet this far above the cell's floor clear the block about to go under them.
const PILLAR_CLEARANCE: f64 = 1.02;

pub enum Outcome {
    /// Hand work under way: bid this steer.
    Busy(Steer),
    /// Nothing left to do with the hands; walk.
    Clear,
    Failed(NavFailure),
}

fn liquid_beside(t: &dyn Terrain, p: BlockPos) -> bool {
    [
        p.above(),
        p.offset(1, 0, 0),
        p.offset(-1, 0, 0),
        p.offset(0, 0, 1),
        p.offset(0, 0, -1),
    ]
    .iter()
    .any(|&q| t.cell(q).is_liquid())
}

/// One tick of hand work on `h`. Dug cells that are open now and a placement whose cell is
/// filled drop off the hop; a filled cell is recorded in `placed`.
pub fn step(
    h: &mut Hop,
    snap: &Snapshot,
    t: &dyn Terrain,
    ticks: &mut u32,
    placed: &mut Vec<BlockPos>,
) -> Outcome {
    let hold = h.from.feet_centre();
    while let Some(&c) = h.dig.first() {
        if t.cell(c).is_solid() {
            break;
        }
        h.dig.remove(0);
        *ticks = 0;
    }
    if let Some(&c) = h.dig.first() {
        // The world changed since planning: never open water or lava onto the body.
        if liquid_beside(t, c) {
            return Outcome::Failed(NavFailure::Stalled);
        }
        *ticks += 1;
        if *ticks > DIG_LIMIT {
            return Outcome::Failed(NavFailure::Stalled);
        }
        let mut s = Steer::walk(hold, false, false);
        s.work = Some(Work::Dig(c));
        return Outcome::Busy(s);
    }
    let Some(at) = h.place else {
        return Outcome::Clear;
    };
    if t.cell(at.cell) != Cell::Open && t.cell(at.cell) != Cell::Water {
        placed.push(at.cell);
        h.place = None;
        *ticks = 0;
        return Outcome::Clear;
    }
    *ticks += 1;
    if *ticks > PLACE_LIMIT {
        return Outcome::Failed(NavFailure::Stalled);
    }
    let me = &snap.me;
    let mut s = Steer::walk(hold, false, false);
    let click = *ticks % CLICK_GAP == 1;
    match h.kind {
        HopKind::Pillar => {
            // Jump straight up and put the block under the feet once they clear it.
            let high = me.pos.y >= f64::from(at.cell.y) + PILLAR_CLEARANCE;
            let centred = (me.pos - hold).horizontal().len() < 0.3;
            s.jump = me.on_ground && centred;
            if high {
                s.work = Some(Work::Place { at, click: true });
            } else {
                s.work = Some(Work::Place { at, click: false });
            }
        }
        _ => s.work = Some(Work::Place { at, click }),
    }
    Outcome::Busy(s)
}

/// Where to look to place `at`: the middle of the face it shares with its support.
pub fn face_point(at: crate::goals::travel::route::Placement) -> Vec3 {
    let c = |b: BlockPos| b.feet_centre() + Vec3::new(0.0, 0.5, 0.0);
    (c(at.cell) + c(at.against)) * 0.5
}
