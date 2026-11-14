//! `reflex_core::goals::World` on Azalea's world, registry and inventory.
//!
//! UNTESTED LIVE: every path here compiles against the pinned Azalea revision but has not
//! run on a server. Riskiest, in order: crafting clicks (`craft`), tossing (`inventory`),
//! the sleeping flag (`entities`), the nearest-block search cost on large empty areas
//! (`blocks`), and the tool tiers (`tools`).

mod blocks;
mod craft;
mod entities;
mod inventory;
pub(crate) mod names;
mod recipes;
mod tools;

use azalea::Client;
use reflex_core::{
    goals::{ItemDrop, MenuStatus, Recipe, ToolNeed, World},
    math::{BlockPos, Vec3},
    snapshot::{Slot, Snapshot},
};

pub use recipes::lookup as recipe_shape;

/// Scaffold blocks (the route planner's throwaway kinds) held in the inventory.
pub fn scaffold_count(bot: &Client) -> u32 {
    reflex_core::goals::travel::route::SCAFFOLD
        .iter()
        .map(|b| inventory::count(bot, b))
        .sum()
}

/// A copy of the player's inventory menu, for pricing digs with the tools at hand.
pub fn inventory_menu(bot: &Client) -> Option<azalea::inventory::Menu> {
    let ecs = bot.ecs.read();
    ecs.get::<azalea::entity::inventory::Inventory>(bot.entity)
        .map(|i| i.inventory_menu.clone())
}

pub struct AzaleaWorld {
    bot: Client,
    crafter: craft::Crafter,
}

impl AzaleaWorld {
    pub fn new(bot: Client) -> Self {
        Self {
            bot,
            crafter: craft::Crafter::default(),
        }
    }
}

impl World for AzaleaWorld {
    fn block_at(&self, p: BlockPos) -> Option<String> {
        blocks::block_at(&self.bot, p)
    }

    fn nearest_block(
        &self,
        kind: &str,
        from: BlockPos,
        radius: i32,
        exclude: &[BlockPos],
    ) -> Option<BlockPos> {
        blocks::nearest(&self.bot, kind, from, radius, exclude)
    }

    fn tool_for(&self, block: &str, _snap: &Snapshot) -> ToolNeed {
        tools::tool_for(&self.bot, block)
    }

    fn count_item(&self, item: &str) -> u32 {
        inventory::count(&self.bot, item)
    }

    fn total_items(&self) -> u32 {
        inventory::total(&self.bot)
    }

    fn item_slot(&self, item: &str) -> Option<Slot> {
        inventory::slot_of(&self.bot, item)
    }

    fn free_slots(&self) -> u32 {
        inventory::free(&self.bot)
    }

    fn recipe(&self, item: &str) -> Option<Recipe> {
        recipes::lookup(item).map(|s| s.recipe())
    }

    fn item_drops(&self, item: Option<&str>, around: Vec3, radius: f64) -> Vec<ItemDrop> {
        entities::drops(&self.bot, item, around, radius)
    }

    fn sleeping(&self) -> bool {
        entities::sleeping(&self.bot)
    }

    fn open_sky(&self, feet: BlockPos) -> bool {
        let t = crate::nav::LiveTerrain::new(&self.bot);
        reflex_core::goals::travel::route::open_sky(&t, feet)
    }

    fn start_craft(&mut self, item: &str, crafts: u32, table: Option<BlockPos>) {
        self.crafter.start(&self.bot, item, crafts, table);
    }

    fn craft_status(&mut self) -> MenuStatus {
        self.crafter.status(&self.bot)
    }

    fn toss(&mut self, item: &str, count: u32) -> u32 {
        inventory::toss(&self.bot, item, count)
    }
}
