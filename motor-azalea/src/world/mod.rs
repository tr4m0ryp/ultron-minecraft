//! `reflex_core::goals::World` on Azalea's world, registry and inventory.
//!
//! Recipes come from the server jar's data pack (`recipes`), read once at bot start.

mod blocks;
mod clicks;
mod craft;
mod entities;
mod furnace;
mod inventory;
pub(crate) mod names;
mod recipes;
mod tools;

use std::sync::Arc;

use azalea::Client;
use reflex_core::{
    crafting::{Recipe, RecipeBook},
    goals::{FurnaceJob, FurnaceSlots, ItemDrop, MenuStatus, ToolNeed, World},
    math::{BlockPos, Vec3},
    snapshot::{Slot, Snapshot},
};

pub use recipes::{book as recipe_book, load as load_recipes, JAR_ENV};

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

/// Main inventory and hotbar by exact item name, counts summed per name.
pub fn inventory_list(bot: &Client) -> Vec<(String, u32)> {
    inventory::list(bot)
}

pub struct AzaleaWorld {
    bot: Client,
    book: Arc<RecipeBook>,
    crafter: craft::Crafter,
    furnace: furnace::Furnacer,
}

impl AzaleaWorld {
    pub fn new(bot: Client) -> Self {
        Self {
            bot,
            book: recipes::book(),
            crafter: craft::Crafter::default(),
            furnace: furnace::Furnacer::default(),
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

    fn inventory(&self) -> Vec<(String, u32)> {
        inventory::list(&self.bot)
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

    fn recipe_book(&self) -> Arc<RecipeBook> {
        self.book.clone()
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

    fn start_craft(&mut self, recipe: &Recipe, crafts: u32, table: Option<BlockPos>) {
        self.crafter.start(&self.bot, recipe, crafts, table);
    }

    fn craft_status(&mut self) -> MenuStatus {
        self.crafter.status(&self.bot)
    }

    fn start_furnace(&mut self, pos: BlockPos, job: &FurnaceJob) {
        self.furnace.start(&self.bot, pos, job);
    }

    fn furnace_status(&mut self) -> MenuStatus {
        self.furnace.status(&self.bot)
    }

    fn furnace_seen(&self) -> Option<FurnaceSlots> {
        self.furnace.seen.clone()
    }

    fn toss(&mut self, item: &str, count: u32) -> u32 {
        inventory::toss(&self.bot, item, count)
    }
}
