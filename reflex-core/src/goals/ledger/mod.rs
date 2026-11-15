//! The ledger of blocks the player placed itself, and of blocks it saw the companion place.
//! A hut the body built is the body's own work: routes and escapes never dig it, and a
//! placement never clears it, unless a goal says so by name. Live, the thought layer never
//! called `protect`, so a route was free to tunnel through the dirt hut the body had just
//! put up. Remembering the placement is automatic instead.
//!
//! A cell is forgotten when it is seen broken or changed (`audit`), so the ledger follows
//! the world. It persists in a text file (`persist`) so a restart keeps it, together with
//! the boxes thought marked with the protect tool (`regions`).

mod persist;
mod regions;
#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use crate::goals::travel::route::Region;
use crate::math::BlockPos;

/// The body refuses to dig within this many blocks (each axis) of a block the companion
/// placed: a wall the companion put up shades its surroundings too.
pub const COMPANION_MARGIN: i32 = 2;

/// Cells a route may not break, for the planner thread (hence `Arc`): the body's own
/// blocks exactly, and everything within the margin of a block the companion placed.
#[derive(Clone, Debug, Default)]
pub struct Keep {
    exact: Arc<HashSet<BlockPos>>,
    near: Arc<HashSet<BlockPos>>,
}

/// Two keep sets are equal when they hold the same cells (a clone is trivially equal).
impl PartialEq for Keep {
    fn eq(&self, other: &Self) -> bool {
        (Arc::ptr_eq(&self.exact, &other.exact) || self.exact == other.exact)
            && (Arc::ptr_eq(&self.near, &other.near) || self.near == other.near)
    }
}

fn within_margin(set: &HashSet<BlockPos>, p: BlockPos) -> bool {
    let m = COMPANION_MARGIN;
    (-m..=m).any(|dx| (-m..=m).any(|dy| (-m..=m).any(|dz| set.contains(&p.offset(dx, dy, dz)))))
}

impl Keep {
    pub fn contains(&self, p: BlockPos) -> bool {
        self.exact.contains(&p) || (!self.near.is_empty() && within_margin(&self.near, p))
    }
    pub fn is_empty(&self) -> bool {
        self.exact.is_empty() && self.near.is_empty()
    }
}

impl FromIterator<BlockPos> for Keep {
    /// A keep set of exact cells with no margin.
    fn from_iter<I: IntoIterator<Item = BlockPos>>(it: I) -> Self {
        Keep {
            exact: Arc::new(it.into_iter().collect()),
            near: Arc::default(),
        }
    }
}

#[derive(Debug, Default)]
struct Inner {
    /// Blocks the body placed, with the kind it saw there.
    own: HashMap<BlockPos, String>,
    /// Blocks the companion was seen placing.
    companion: HashMap<BlockPos, String>,
    keep: Option<Keep>,
    /// Boxes marked with the protect tool, mirrored from the knobs (`sync_regions`).
    regions: Vec<Region>,
    regions_restored: bool,
    dirty: bool,
    path: Option<PathBuf>,
}

/// Shared between the goal runner and the adapter's tick loop (one thread).
#[derive(Clone, Debug, Default)]
pub struct Ledger(Rc<RefCell<Inner>>);

impl Ledger {
    /// An in-memory ledger that persists nowhere.
    pub fn new() -> Self {
        Self::default()
    }

    /// A ledger persisted at `path`: loaded now when the file exists, written by `flush`.
    pub fn open(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let l = Self::default();
        {
            let mut guard = l.0.borrow_mut();
            let i = &mut *guard;
            persist::load(&path, &mut i.own, &mut i.companion, &mut i.regions);
            i.path = Some(path);
        }
        l
    }

    fn touch(i: &mut Inner) {
        i.keep = None;
        i.dirty = true;
    }

    /// The body placed `kind` at `p` (the server showed it there).
    pub fn remember(&self, p: BlockPos, kind: &str) {
        let mut i = self.0.borrow_mut();
        if i.own.get(&p).map(String::as_str) != Some(kind) {
            i.own.insert(p, kind.to_string());
            Self::touch(&mut i);
        }
    }

    /// The companion was seen placing `kind` at `p`.
    pub fn remember_companion(&self, p: BlockPos, kind: &str) {
        let mut i = self.0.borrow_mut();
        if i.companion.get(&p).map(String::as_str) != Some(kind) {
            i.companion.insert(p, kind.to_string());
            Self::touch(&mut i);
        }
    }

    /// Drops `p`; returns whether it was remembered.
    pub fn forget(&self, p: BlockPos) -> bool {
        let mut i = self.0.borrow_mut();
        let gone = i.own.remove(&p).is_some() | i.companion.remove(&p).is_some();
        if gone {
            Self::touch(&mut i);
        }
        gone
    }

    /// `p` holds a block somebody placed (the body or the companion): never cleared by a
    /// placement without `replace`.
    pub fn is_placed(&self, p: BlockPos) -> bool {
        let i = self.0.borrow();
        i.own.contains_key(&p) || i.companion.contains_key(&p)
    }

    /// Routes, escapes and mining must not break `p`: a block the body placed, or any cell
    /// within the margin of one the companion placed.
    pub fn blocks_dig(&self, p: BlockPos) -> bool {
        let i = self.0.borrow();
        let m = COMPANION_MARGIN;
        i.own.contains_key(&p)
            || (!i.companion.is_empty()
                && (-m..=m).any(|dx| {
                    (-m..=m).any(|dy| {
                        (-m..=m).any(|dz| i.companion.contains_key(&p.offset(dx, dy, dz)))
                    })
                }))
    }

    /// What a route must not break, for the planner thread. Rebuilt only after a change,
    /// so asking each tick is cheap.
    pub fn keep(&self) -> Keep {
        let mut i = self.0.borrow_mut();
        if let Some(k) = &i.keep {
            return k.clone();
        }
        let k = Keep {
            exact: Arc::new(i.own.keys().copied().collect()),
            near: Arc::new(i.companion.keys().copied().collect()),
        };
        i.keep = Some(k.clone());
        k
    }

    pub fn len(&self) -> usize {
        let i = self.0.borrow();
        i.own.len() + i.companion.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The body's own cells, for tests and reports.
    pub fn own_cells(&self) -> Vec<(BlockPos, String)> {
        let mut v: Vec<_> = self
            .0
            .borrow()
            .own
            .iter()
            .map(|(p, k)| (*p, k.clone()))
            .collect();
        v.sort_by_key(|(p, _)| *p);
        v
    }

    /// Cells of the ledger within `radius` (each axis) of `near`.
    pub fn cells_near(&self, near: BlockPos, radius: i32) -> Vec<(BlockPos, String)> {
        let i = self.0.borrow();
        i.own
            .iter()
            .chain(i.companion.iter())
            .filter(|(p, _)| {
                (p.x - near.x).abs() <= radius
                    && (p.y - near.y).abs() <= radius
                    && (p.z - near.z).abs() <= radius
            })
            .map(|(p, k)| (*p, k.clone()))
            .collect()
    }

    /// Forgets the cells within `radius` of `near` that `read` shows broken or changed.
    /// `read` returns the block name at a cell, `None` when it is not loaded (kept).
    /// Returns how many were forgotten.
    pub fn audit(
        &self,
        near: BlockPos,
        radius: i32,
        read: impl Fn(BlockPos) -> Option<String>,
    ) -> usize {
        let stale: Vec<BlockPos> = self
            .cells_near(near, radius)
            .into_iter()
            .filter(|(p, kind)| read(*p).is_some_and(|now| now != *kind))
            .map(|(p, _)| p)
            .collect();
        for p in &stale {
            self.forget(*p);
        }
        stale.len()
    }

    /// Writes the file when something changed since the last write. Errors are returned for
    /// the caller to log; the ledger stays usable in memory.
    pub fn flush(&self) -> std::io::Result<()> {
        let mut i = self.0.borrow_mut();
        let Some(path) = i.path.clone() else {
            return Ok(());
        };
        if !i.dirty {
            return Ok(());
        }
        persist::save(&path, &i.own, &i.companion, &i.regions)?;
        i.dirty = false;
        Ok(())
    }
}
