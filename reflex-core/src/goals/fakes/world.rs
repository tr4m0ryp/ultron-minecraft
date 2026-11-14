use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::goals::ports::{kind_matches, ItemDrop, MenuStatus, Recipe, ToolNeed, World};
use crate::math::{BlockPos, Vec3};
use crate::snapshot::{Slot, Snapshot};

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
    pub recipes: HashMap<String, Recipe>,
    pub drops: Vec<ItemDrop>,
    pub sleeping: bool,
    /// `craft_status` calls that report Busy before the craft lands.
    pub craft_ticks: u32,
    /// Make the next craft fail with this reason.
    pub craft_error: Option<String>,
    pub tossed: Vec<(String, u32)>,
    craft: Option<(String, u32, u32)>,
    status: Option<MenuStatus>,
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

    fn land_craft(&mut self, item: &str, crafts: u32) {
        let Some(r) = self.recipes.get(item).cloned() else {
            self.status = Some(MenuStatus::Failed("unknown recipe".into()));
            return;
        };
        for (ing, n) in &r.ingredients {
            self.take_item(ing, n * crafts);
        }
        self.add_item(item, r.yields * crafts);
        self.status = Some(MenuStatus::Done);
    }
}

#[derive(Clone, Debug, Default)]
pub struct FakeWorld(pub Rc<RefCell<FakeWorldState>>);

impl FakeWorld {
    pub fn state(&self) -> std::cell::RefMut<'_, FakeWorldState> {
        self.0.borrow_mut()
    }
}

impl World for FakeWorld {
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
        self.0
            .borrow()
            .blocks
            .iter()
            .filter(|(p, n)| kind_matches(kind, n) && !exclude.contains(p))
            .map(|(p, _)| *p)
            .filter(|p| d(p) <= radius as f64)
            .min_by(|a, b| {
                d(a).total_cmp(&d(b))
                    .then(a.x.cmp(&b.x))
                    .then(a.z.cmp(&b.z))
            })
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
        s.inventory
            .iter()
            .filter(|(n, _)| kind_matches(item, n))
            .map(|(_, c)| c)
            .sum()
    }

    fn item_slot(&self, item: &str) -> Option<Slot> {
        let s = self.0.borrow();
        let held = |n: &String| s.inventory.iter().any(|(k, c)| k == n && *c > 0);
        s.slots
            .iter()
            .filter(|(n, _)| kind_matches(item, n) && held(n))
            .map(|(_, slot)| *slot)
            .min()
    }

    fn free_slots(&self) -> u32 {
        self.0.borrow().free_slots
    }

    fn recipe(&self, item: &str) -> Option<Recipe> {
        self.0.borrow().recipes.get(item).cloned()
    }

    fn item_drops(&self, item: Option<&str>, around: Vec3, radius: f64) -> Vec<ItemDrop> {
        self.0
            .borrow()
            .drops
            .iter()
            .filter(|d| item.is_none_or(|k| kind_matches(k, &d.item)))
            .filter(|d| d.pos.dist(around) <= radius)
            .cloned()
            .collect()
    }

    fn sleeping(&self) -> bool {
        self.0.borrow().sleeping
    }

    fn start_craft(&mut self, item: &str, crafts: u32, _table: Option<BlockPos>) {
        let mut s = self.0.borrow_mut();
        if let Some(e) = s.craft_error.take() {
            s.status = Some(MenuStatus::Failed(e));
            return;
        }
        let ticks = s.craft_ticks;
        s.craft = Some((item.to_string(), crafts, ticks));
        s.status = Some(MenuStatus::Busy);
    }

    fn craft_status(&mut self) -> MenuStatus {
        let mut s = self.0.borrow_mut();
        if let Some((item, crafts, left)) = s.craft.take() {
            if left == 0 {
                s.land_craft(&item, crafts);
            } else {
                s.craft = Some((item, crafts, left - 1));
            }
        }
        s.status.clone().unwrap_or(MenuStatus::Idle)
    }

    fn toss(&mut self, item: &str, count: u32) -> u32 {
        let mut s = self.0.borrow_mut();
        let n = s.take_item(item, count);
        s.tossed.push((item.to_string(), n));
        n
    }
}
