//! The world port in memory: named blocks, a name-counted inventory, a recipe book, and
//! crafting and furnace menus that land after a few status polls.

mod menus;
mod state;
mod stations;

pub use state::FakeWorldState;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::crafting::{Recipe, RecipeBook};
use crate::goals::ports::{
    kind_matches, name_matches, FurnaceJob, FurnaceSlots, ItemDrop, MenuStatus, StationJob,
    ToolNeed, World,
};
use crate::math::{BlockPos, Vec3};
use crate::snapshot::{Slot, Snapshot};

#[derive(Clone, Debug, Default)]
pub struct FakeWorld(pub Rc<RefCell<FakeWorldState>>);

impl FakeWorld {
    pub fn state(&self) -> std::cell::RefMut<'_, FakeWorldState> {
        self.0.borrow_mut()
    }
}

impl World for FakeWorld {
    fn full_name(&self, kind: &str) -> bool {
        self.0.borrow().full_name(kind)
    }

    fn block_at(&self, p: BlockPos) -> Option<String> {
        let s = self.0.borrow();
        let ground = s.ground_y.is_some_and(|y| p.y <= y);
        let default = if ground { "stone" } else { "air" };
        Some(s.blocks.get(&p).cloned().unwrap_or(default.into()))
    }

    fn nearest_block(
        &self,
        kind: &str,
        from: BlockPos,
        radius: i32,
        exclude: &[BlockPos],
    ) -> Option<BlockPos> {
        let d = |p: &BlockPos| {
            let (x, y, z) = (p.x - from.x, p.y - from.y, p.z - from.z);
            ((x * x + y * y + z * z) as f64).sqrt()
        };
        let s = self.0.borrow();
        let full = s.full_name(kind);
        s.blocks
            .iter()
            .filter(|(p, n)| name_matches(kind, n, full) && !exclude.contains(p))
            .map(|(p, _)| *p)
            .filter(|p| d(p) <= radius as f64)
            .min_by(|a, b| {
                d(a).total_cmp(&d(b))
                    .then(a.x.cmp(&b.x))
                    .then(a.z.cmp(&b.z))
            })
    }

    fn water_source(&self, from: BlockPos, radius: i32) -> Option<BlockPos> {
        self.nearest_block("water", from, radius, &[])
    }

    fn tool_for(&self, block: &str, _snap: &Snapshot) -> ToolNeed {
        let s = self.0.borrow();
        s.tools
            .iter()
            .find(|(k, _)| kind_matches(k, block) || kind_matches(block, k))
            .map_or(ToolNeed::Hand, |(_, t)| t.clone())
    }

    fn count_item(&self, item: &str) -> u32 {
        let s = self.0.borrow();
        let full = s.full_name(item);
        s.inventory
            .iter()
            .filter(|(n, _)| name_matches(item, n, full))
            .map(|(_, c)| c)
            .sum()
    }

    fn inventory(&self) -> Vec<(String, u32)> {
        let s = self.0.borrow();
        let mut out: Vec<(String, u32)> = Vec::new();
        for (n, c) in s.inventory.iter().filter(|(_, c)| *c > 0) {
            match out.iter_mut().find(|(k, _)| k == n) {
                Some((_, k)) => *k += c,
                None => out.push((n.clone(), *c)),
            }
        }
        out
    }

    fn total_items(&self) -> u32 {
        self.0.borrow().inventory.iter().map(|(_, c)| c).sum()
    }

    fn item_slot(&self, item: &str) -> Option<Slot> {
        let s = self.0.borrow();
        let held = |n: &String| s.inventory.iter().any(|(k, c)| k == n && *c > 0);
        let full = s.full_name(item);
        s.slots
            .iter()
            .filter(|(n, _)| name_matches(item, n, full) && held(n))
            .map(|(_, slot)| *slot)
            .min()
    }

    fn free_slots(&self) -> u32 {
        self.0.borrow().free_slots
    }

    fn recipe_book(&self) -> Arc<RecipeBook> {
        self.0.borrow().book.clone()
    }

    fn item_drops(&self, item: Option<&str>, around: Vec3, radius: f64) -> Vec<ItemDrop> {
        let s = self.0.borrow();
        s.drops
            .iter()
            .filter(|d| item.is_none_or(|k| name_matches(k, &d.item, s.full_name(k))))
            .filter(|d| d.pos.dist(around) <= radius)
            .cloned()
            .collect()
    }

    fn sleeping(&self) -> bool {
        self.0.borrow().sleeping
    }

    fn open_sky(&self, _feet: BlockPos) -> bool {
        !self.0.borrow().no_sky
    }

    fn start_craft(&mut self, recipe: &Recipe, crafts: u32, _table: Option<BlockPos>) {
        self.0.borrow_mut().start_craft(recipe, crafts);
    }

    fn craft_status(&mut self) -> MenuStatus {
        self.0.borrow_mut().craft_status()
    }

    fn start_furnace(&mut self, pos: BlockPos, job: &FurnaceJob) {
        let mut s = self.0.borrow_mut();
        let ticks = s.craft_ticks;
        s.furnace_job = Some((pos, job.clone(), ticks));
        s.furnace_status = Some(MenuStatus::Busy);
    }

    fn furnace_status(&mut self) -> MenuStatus {
        self.0.borrow_mut().furnace_status()
    }

    fn furnace_seen(&self) -> Option<FurnaceSlots> {
        self.0.borrow().furnace_seen.clone()
    }

    fn start_station(&mut self, pos: BlockPos, job: &StationJob) {
        let mut s = self.0.borrow_mut();
        let ticks = s.craft_ticks;
        s.station_job = Some((pos, job.clone(), ticks));
        s.station_status = Some(MenuStatus::Busy);
    }

    fn station_status(&mut self) -> MenuStatus {
        self.0.borrow_mut().station_status()
    }

    fn toss(&mut self, item: &str, count: u32) -> u32 {
        let mut s = self.0.borrow_mut();
        let n = s.take_item(item, count);
        s.tossed.push((item.to_string(), n));
        n
    }
}
