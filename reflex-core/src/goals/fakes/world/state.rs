//! The fake world's state: blocks, a name-counted inventory, recipes and menus in flight.

use std::collections::HashMap;
use std::sync::Arc;

use crate::crafting::{Recipe, RecipeBook};
use crate::goals::ports::{kind_matches, FurnaceJob, FurnaceSlots, ItemDrop, MenuStatus, ToolNeed};
use crate::math::BlockPos;
use crate::snapshot::Slot;

/// Named blocks and a name-counted inventory. Blocks not listed read as "air".
#[derive(Debug, Default)]
pub struct FakeWorldState {
    pub blocks: HashMap<BlockPos, String>,
    /// Unlisted blocks at or below this height read as "stone" (the arena floor).
    pub ground_y: Option<i32>,
    pub inventory: Vec<(String, u32)>,
    pub slots: HashMap<String, Slot>,
    pub free_slots: u32,
    /// Tool needs by block kind; unlisted kinds break by hand.
    pub tools: HashMap<String, ToolNeed>,
    pub book: Arc<RecipeBook>,
    pub drops: Vec<ItemDrop>,
    pub sleeping: bool,
    /// Read every cell as roofed over (no open sky).
    pub no_sky: bool,
    /// `craft_status` calls that report Busy before the craft lands.
    pub craft_ticks: u32,
    /// Make the next craft fail with this reason.
    pub craft_error: Option<String>,
    pub tossed: Vec<(String, u32)>,
    /// Furnaces by position: what each holds, and the burn left in quarter smelts.
    pub furnaces: HashMap<BlockPos, (FurnaceSlots, u32)>,
    pub(super) craft: Option<(Recipe, u32, u32)>,
    pub(super) status: Option<MenuStatus>,
    pub(super) furnace_job: Option<(BlockPos, FurnaceJob, u32)>,
    pub(super) furnace_status: Option<MenuStatus>,
    pub(super) furnace_seen: Option<FurnaceSlots>,
}

impl FakeWorldState {
    pub fn add_item(&mut self, name: &str, n: u32) {
        match self.inventory.iter_mut().find(|(k, _)| k == name) {
            Some((_, c)) => *c += n,
            None => self.inventory.push((name.to_string(), n)),
        }
    }

    /// Removes up to `n` of the stacks matching `kind`; returns how many were removed.
    pub fn take_item(&mut self, kind: &str, n: u32) -> u32 {
        let mut left = n;
        for (name, c) in self.inventory.iter_mut() {
            if left > 0 && kind_matches(kind, name) {
                let t = (*c).min(left);
                *c -= t;
                left -= t;
            }
        }
        self.inventory.retain(|(_, c)| *c > 0);
        n - left
    }
}
