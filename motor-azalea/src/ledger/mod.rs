//! Keeping the goal layer's ledger of placed blocks true to the server: where the file
//! lives, which placements the server accepted, which remembered cells were since broken,
//! and which blocks the companion put up (`companion`).

mod companion;

use std::path::PathBuf;

use azalea::Client;
use reflex_core::{
    channels::MainCmd,
    goals::{items::replaceable, Ledger},
    math::BlockPos,
    snapshot::Snapshot,
    MotorOutput,
};

use crate::world::names::block_name;
use companion::CompanionWatch;

/// Names the ledger file. `off` (or `none`, `0`) keeps the ledger in memory only. Unset: the
/// play state directory, `logs/play/placed.txt` under the working directory. Staged runs
/// set their own file, never the play one (`tools/live/bot.sh` does).
pub const LEDGER_ENV: &str = "ULTRON_LEDGER";
const DEFAULT_PATH: &str = "logs/play/placed.txt";

/// Ticks between audits of remembered cells near the body.
const AUDIT_EVERY: u64 = 20;
const AUDIT_RADIUS: i32 = 24;
/// Ticks a click may take before the server's answer is given up on.
const CLICK_TICKS: u64 = 60;

pub fn path() -> Option<PathBuf> {
    match std::env::var(LEDGER_ENV) {
        Ok(v) if matches!(v.trim(), "off" | "none" | "0") => None,
        Ok(v) if !v.trim().is_empty() => Some(PathBuf::from(v.trim())),
        _ => Some(PathBuf::from(DEFAULT_PATH)),
    }
}

/// The ledger for this run: loaded from its file when it has one.
pub fn open() -> Ledger {
    match path() {
        Some(p) => {
            let l = Ledger::open(&p);
            println!("LEDGER {} cells from {}", l.len(), p.display());
            l
        }
        None => Ledger::new(),
    }
}

fn block_at(bot: &Client, p: BlockPos) -> Option<String> {
    let world = bot.world().ok()?;
    let w = world.read();
    w.get_block_state(azalea::BlockPos::new(p.x, p.y, p.z))
        .map(block_name)
}

fn flush(ledger: &Ledger) {
    if let Err(e) = ledger.flush() {
        eprintln!("ledger not written: {e}");
    }
}

/// What the tick loop does for the ledger each tick.
#[derive(Default)]
pub struct Watch {
    /// Cells a place click went to, with the tick of the click: whatever the server shows
    /// there afterwards is a block the body placed, however the click came about (a place
    /// goal, a route's scaffold, a reflex).
    clicks: Vec<(BlockPos, u64)>,
    companion: CompanionWatch,
}

impl Watch {
    pub fn tick(&mut self, bot: &Client, ledger: &Ledger, snap: &Snapshot, out: &MotorOutput) {
        let now = snap.tick;
        if let Some(MainCmd::PlaceAt { cell, .. }) = out.main {
            if !self.clicks.iter().any(|(c, _)| *c == cell) {
                self.clicks.push((cell, now));
            }
        }
        let mut confirmed: Vec<(BlockPos, String)> = Vec::new();
        self.clicks.retain(|&(cell, at)| {
            if now.saturating_sub(at) > CLICK_TICKS {
                return false;
            }
            match block_at(bot, cell) {
                Some(name) if now > at && !replaceable(&name) => {
                    ledger.remember(cell, &name);
                    confirmed.push((cell, name));
                    false
                }
                _ => true,
            }
        });
        let pending: Vec<BlockPos> = self.clicks.iter().map(|(c, _)| *c).collect();
        self.companion.tick(bot, ledger, snap, &pending);
        if now.is_multiple_of(AUDIT_EVERY) {
            let feet = snap.me.pos.block();
            ledger.audit(feet, AUDIT_RADIUS, |p| block_at(bot, p));
        }
        // Written in the tick the server confirms a block, not at the next audit: a bot
        // killed in between (up to 20 ticks) forgot the block and was free to dig it later.
        // Nothing is written on a tick that changed nothing. The line is printed after the
        // write, so it means "on disk".
        flush(ledger);
        for (c, name) in &confirmed {
            println!("LEDGER own {} {} {} {name} t={now}", c.x, c.y, c.z);
        }
    }
}
