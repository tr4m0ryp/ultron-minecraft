//! Planning one round of a craft goal: the plan from the inventory, and its first step set
//! up as a phase (a grid craft, a walk to a station, a station to put down first).

use super::brew::Brewing;
use super::furnace::Smelting;
use super::smith::Smithing;
use super::station::{self, Placing};
use super::{Craft, Phase, MAX_STEPS};
use crate::crafting::{
    plan, PlanStep, Stations, BREWING_STAND, FURNACE, SMITHING_TABLE, TABLE,
};
use crate::goals::task::GoalCx;

impl Craft {
    /// Plans from the inventory and sets up the first step.
    fn replan(&mut self, cx: &mut GoalCx, wanted: u32) -> Result<(), String> {
        self.steps += 1;
        if self.steps > MAX_STEPS {
            return Err(format!("{} took more than {MAX_STEPS} steps", self.item));
        }
        let book = cx.world.recipe_book();
        if book.is_empty() {
            return Err("no recipe data: the server jar was not found at bot start".into());
        }
        let table = station::find(cx, TABLE);
        let furnace = station::find(cx, FURNACE);
        let near = Stations {
            table: table.is_some(),
            furnace: furnace.is_some(),
        };
        let inv = cx.world.inventory();
        let p = plan(&book, &inv, &self.item, wanted, near, self.smelt_only)?;
        let steps: Vec<String> = p.steps.iter().map(PlanStep::describe).collect();
        cx.doing(format!("plan: {}", steps.join(", then ")));
        let held = |cx: &GoalCx, b: &str| cx.world.count_item(b) > 0;
        self.phase = match p.steps.into_iter().next() {
            None => return Err(format!("nothing to do for {}", self.item)),
            Some(PlanStep::Craft { recipe, crafts }) => {
                if cx.world.free_slots() == 0 {
                    return Err(format!("inventory full; no room for {}", recipe.result));
                }
                match (recipe.needs_table(), table) {
                    (false, _) => Phase::Craft(recipe, crafts, None),
                    (true, Some(t)) => Phase::Walk(t, recipe, crafts),
                    (true, None) if held(cx, TABLE) => Phase::Place(Placing::new(TABLE)),
                    (true, None) => {
                        return Err(format!("{} needs a crafting table", recipe.result))
                    }
                }
            }
            Some(PlanStep::Smelt {
                recipe,
                count,
                fuel,
            }) => match furnace {
                Some(f) => Phase::Smelt(Smelting::new(f, recipe, count, fuel)),
                None if held(cx, FURNACE) => Phase::Place(Placing::new(FURNACE)),
                None => return Err(format!("{} needs a furnace", recipe.result)),
            },
        };
        Ok(())
    }
}
