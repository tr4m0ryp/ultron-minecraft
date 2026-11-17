//! The fake world's state: blocks, a name-counted inventory, recipes and menus in flight.

use std::collections::HashMap;
use std::sync::Arc;

use super::stations::Stand;
use crate::crafting::{Recipe, RecipeBook};
use crate::goals::ports::{
    name_matches, FurnaceJob, FurnaceSlots, ItemDrop, MenuStatus, StationJob, ToolNeed,
};
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
    /// Names the registry would know beyond the ones this world holds (see `full_name`).
    pub names: Vec<String>,
    pub sleeping: bool,
    /// Right clicks on a bed (by day they only set the spawn point).
    pub bed_clicks: u32,
    /// Read every cell as roofed over (no open sky).
    pub no_sky: bool,
    /// `craft_status` calls that report Busy before the craft lands.
    pub craft_ticks: u32,
    /// Make the next craft fail with this reason.
    pub craft_error: Option<String>,
    pub tossed: Vec<(String, u32)>,
    /// Furnaces by position: what each holds, and the burn left in quarter smelts.
    pub furnaces: HashMap<BlockPos, (FurnaceSlots, u32)>,
    /// Furnace visits that leave the output in the furnace (live: clicks before the
    /// server sent the slots).
    pub furnace_output_missed: u32,
    /// Brewing stands by position.
    pub stands: HashMap<BlockPos, Stand>,
    pub(super) station_job: Option<(BlockPos, StationJob, u32)>,
    pub(super) station_status: Option<MenuStatus>,
    pub(super) craft: Option<(Recipe, u32, u32)>,
    pub(super) status: Option<MenuStatus>,
    pub(super) furnace_job: Option<(BlockPos, FurnaceJob, u32)>,
    pub(super) furnace_status: Option<MenuStatus>,
    pub(super) furnace_seen: Option<FurnaceSlots>,
}

/// Words that name a family of blocks or items and nothing by themselves.
const FAMILIES: [&str; 12] = [
    "bed", "log", "planks", "wool", "door", "ore", "pickaxe", "axe", "sword", "shovel", "hoe",
    "slab",
];

impl FakeWorldState {
    /// The real adapter asks the game's registries. Here every name is a full one except
    /// the family words above, so a test world matches by exact name as the live one does.
    pub fn full_name(&self, kind: &str) -> bool {
        !FAMILIES.contains(&kind) || self.names.iter().any(|n| n == kind)
    }

    pub fn add_item(&mut self, name: &str, n: u32) {
        match self.inventory.iter_mut().find(|(k, _)| k == name) {
            Some((_, c)) => *c += n,
            None => self.inventory.push((name.to_string(), n)),
        }
    }

    /// Removes up to `n` of the stacks matching `kind`; returns how many were removed.
    pub fn take_item(&mut self, kind: &str, n: u32) -> u32 {
        let mut left = n;
        let full = self.full_name(kind);
        for (name, c) in self.inventory.iter_mut() {
            if left > 0 && name_matches(kind, name, full) {
                let t = (*c).min(left);
                *c -= t;
                left -= t;
            }
        }
        self.inventory.retain(|(_, c)| *c > 0);
        n - left
    }
}
