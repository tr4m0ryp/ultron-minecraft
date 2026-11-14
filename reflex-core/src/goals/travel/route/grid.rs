//! An in-memory terrain for tests and offline runs: stone at or below `ground`, air above,
//! with any cell overridden.

use std::cell::RefCell;
use std::collections::HashMap;

use super::cell::{Cell, Terrain};
use crate::math::BlockPos;

#[derive(Debug, Default)]
pub struct Grid {
    pub ground: i32,
    pub cells: RefCell<HashMap<BlockPos, Cell>>,
}

impl Grid {
    pub fn new(ground: i32) -> Self {
        Self {
            ground,
            cells: RefCell::default(),
        }
    }

    pub fn set(&self, p: BlockPos, c: Cell) {
        self.cells.borrow_mut().insert(p, c);
    }

    /// Fills the box between two corners (inclusive, any order).
    pub fn fill(&self, a: BlockPos, b: BlockPos, c: Cell) {
        for x in a.x.min(b.x)..=a.x.max(b.x) {
            for y in a.y.min(b.y)..=a.y.max(b.y) {
                for z in a.z.min(b.z)..=a.z.max(b.z) {
                    self.set(BlockPos::new(x, y, z), c);
                }
            }
        }
    }
}

impl Terrain for Grid {
    fn cell(&self, p: BlockPos) -> Cell {
        if let Some(c) = self.cells.borrow().get(&p) {
            return *c;
        }
        if p.y <= self.ground {
            Cell::STONE
        } else {
            Cell::Open
        }
    }
}
