//! Crafting and furnace menus in the fake world: a craft takes its inputs and adds its
//! result; a furnace visit cooks what the burn allows, hands over the output and loads.

use super::state::FakeWorldState;
use crate::crafting::{fuel_quarters, Recipe, Shape};
use crate::goals::ports::{FurnaceSlots, MenuStatus};

impl FakeWorldState {
    pub(super) fn start_craft(&mut self, recipe: &Recipe, crafts: u32) {
        if let Some(e) = self.craft_error.take() {
            self.status = Some(MenuStatus::Failed(e));
            return;
        }
        self.craft = Some((recipe.clone(), crafts, self.craft_ticks));
        self.status = Some(MenuStatus::Busy);
    }

    pub(super) fn craft_status(&mut self) -> MenuStatus {
        if let Some((r, crafts, left)) = self.craft.take() {
            if left == 0 {
                self.land_craft(&r, crafts);
            } else {
                self.craft = Some((r, crafts, left - 1));
            }
        }
        self.status.clone().unwrap_or(MenuStatus::Idle)
    }

    fn held(&self, accepts: impl Fn(&str) -> bool) -> u32 {
        let s = self.inventory.iter().filter(|(n, _)| accepts(n));
        s.map(|(_, c)| c).sum()
    }

    fn land_craft(&mut self, r: &Recipe, crafts: u32) {
        for (ing, units) in r.inputs() {
            if self.held(|n| ing.accepts(n)) < units * crafts {
                self.status = Some(MenuStatus::Failed(format!("ran out of {}", ing.0[0])));
                return;
            }
        }
        for (ing, units) in r.inputs() {
            let mut left = units * crafts;
            for name in ing.0.iter() {
                left -= self.take_item(name, left);
            }
        }
        self.add_item(&r.result, r.count * crafts);
        if !self.slots.contains_key(&r.result) {
            let used: Vec<_> = self.slots.values().copied().collect();
            if let Some(free) = (0..36).find(|s| !used.contains(s)) {
                self.slots.insert(r.result.clone(), free);
            }
        }
        self.status = Some(MenuStatus::Done);
    }

    pub(super) fn furnace_status(&mut self) -> MenuStatus {
        if let Some((pos, job, left)) = self.furnace_job.take() {
            if left > 0 {
                self.furnace_job = Some((pos, job, left - 1));
            } else {
                let (mut slots, mut burn) = self.furnaces.remove(&pos).unwrap_or_default();
                self.cook(&mut slots, &mut burn);
                if self.furnace_output_missed > 0 {
                    self.furnace_output_missed -= 1;
                } else if let Some((item, n)) = slots.output.take() {
                    self.add_item(&item, n);
                }
                if let Some((item, n)) = job.input {
                    let t = self.take_item(&item, n);
                    let had = slots.input.as_ref().map_or(0, |(_, c)| *c);
                    slots.input = Some((item, had + t));
                }
                if let Some((item, n)) = job.fuel {
                    let t = self.take_item(&item, n);
                    let had = slots.fuel.as_ref().map_or(0, |(_, c)| *c);
                    slots.fuel = Some((item, had + t));
                }
                self.furnace_seen = Some(slots.clone());
                self.furnaces.insert(pos, (slots, burn));
                self.furnace_status = Some(MenuStatus::Done);
            }
        }
        self.furnace_status.clone().unwrap_or(MenuStatus::Idle)
    }

    /// Smelts everything the burn and fuel cover, as if the cooking time had passed.
    fn cook(&self, slots: &mut FurnaceSlots, burn: &mut u32) {
        while let Some((item, n)) = slots.input.clone().filter(|(_, n)| *n > 0) {
            let result = self.book.all().find_map(|r| match &r.shape {
                Shape::Smelt { input, .. } if input.accepts(&item) => Some(r.result.clone()),
                _ => None,
            });
            let Some(result) = result else {
                return;
            };
            if *burn < 4 {
                match slots.fuel.as_mut().filter(|(_, c)| *c > 0) {
                    Some((f, c)) => {
                        *c -= 1;
                        *burn += fuel_quarters(f);
                    }
                    None => return,
                }
                slots.fuel = slots.fuel.take().filter(|(_, c)| *c > 0);
                continue;
            }
            *burn -= 4;
            slots.input = (n > 1).then(|| (item.clone(), n - 1));
            let made = slots.output.as_ref().map_or(0, |(_, c)| *c) + 1;
            slots.output = Some((result, made));
        }
    }
}
