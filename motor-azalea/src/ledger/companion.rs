//! Seeing the companion place blocks. The body compares the blocks around the companion
//! with what it saw a moment ago: a cell that was open and is now a solid block, within the
//! companion's reach, that the body did not place itself, was placed by the companion.
//! (Block-update packets would say the same, but they arrive after the client world already
//! changed, so they cannot tell a new block from a resent one.)

use azalea::Client;
use reflex_core::{
    goals::Ledger,
    math::{BlockPos, Vec3},
    snapshot::{BlockKind, Snapshot},
};

use crate::sense::block_kind::classify;

/// Box scanned around the companion: horizontal radius, blocks below and above its feet.
const RH: i32 = 6;
const BELOW: i32 = 3;
const ABOVE: i32 = 5;
const W: i32 = 2 * RH + 1;
const H: i32 = BELOW + ABOVE + 1;
/// Farthest a placed block may be from the companion's eye and count as its own.
const REACH: f64 = 6.5;
const SCAN_EVERY: u64 = 2;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Seen {
    Unloaded,
    Open,
    Placed,
}

struct Scan {
    origin: BlockPos,
    cells: Vec<Seen>,
}

impl Scan {
    fn index(&self, p: BlockPos) -> Option<usize> {
        let (dx, dy, dz) = (
            p.x - self.origin.x,
            p.y - self.origin.y,
            p.z - self.origin.z,
        );
        ((0..W).contains(&dx) && (0..H).contains(&dy) && (0..W).contains(&dz))
            .then(|| ((dy * W + dz) * W + dx) as usize)
    }

    fn at(&self, p: BlockPos) -> Seen {
        self.index(p).map_or(Seen::Unloaded, |i| self.cells[i])
    }
}

#[derive(Default)]
pub struct CompanionWatch {
    prev: Option<Scan>,
}

impl CompanionWatch {
    /// `ours`: cells the body just clicked, never credited to the companion.
    pub fn tick(&mut self, bot: &Client, ledger: &Ledger, snap: &Snapshot, ours: &[BlockPos]) {
        let Some(c) = snap.companion.as_ref() else {
            self.prev = None;
            return;
        };
        if !snap.tick.is_multiple_of(SCAN_EVERY) {
            return;
        }
        let now = scan(bot, c.pos.block());
        if let Some(prev) = &self.prev {
            for p in appeared(prev, &now, c.pos + Vec3::new(0.0, 1.62, 0.0)) {
                if ours.contains(&p) || ledger.is_placed(p) {
                    continue;
                }
                if let Some(name) = super::block_at(bot, p) {
                    ledger.remember_companion(p, &name);
                }
            }
        }
        self.prev = Some(now);
    }
}

fn scan(bot: &Client, feet: BlockPos) -> Scan {
    let origin = BlockPos::new(feet.x - RH, feet.y - BELOW, feet.z - RH);
    let mut cells = vec![Seen::Unloaded; (W * W * H) as usize];
    if let Ok(world) = bot.world() {
        let w = world.read();
        let mut i = 0;
        for dy in 0..H {
            for dz in 0..W {
                for dx in 0..W {
                    let p = azalea::BlockPos::new(origin.x + dx, origin.y + dy, origin.z + dz);
                    cells[i] = match w.get_block_state(p).map(classify) {
                        None | Some(BlockKind::Unknown) => Seen::Unloaded,
                        Some(BlockKind::Solid | BlockKind::SolidOpen) => Seen::Placed,
                        Some(_) => Seen::Open,
                    };
                    i += 1;
                }
            }
        }
    }
    Scan { origin, cells }
}

/// Cells open in `prev` and solid in `now`, within the companion's reach of `eye`.
fn appeared(prev: &Scan, now: &Scan, eye: Vec3) -> Vec<BlockPos> {
    let mut out = Vec::new();
    for dy in 0..H {
        for dz in 0..W {
            for dx in 0..W {
                let p = BlockPos::new(now.origin.x + dx, now.origin.y + dy, now.origin.z + dz);
                let reach = p.feet_centre() + Vec3::new(0.0, 0.5, 0.0);
                if now.at(p) == Seen::Placed && prev.at(p) == Seen::Open && reach.dist(eye) <= REACH
                {
                    out.push(p);
                }
            }
        }
    }
    out
}
