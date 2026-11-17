//! Planning one round of a craft goal: the plan from the inventory, and its first step set
//! up as a phase (a grid craft, a walk to a station, a station to put down first).

use super::brew::Brewing;
use super::fill::Filling;
use super::furnace::Smelting;
use super::smith::Smithing;
use super::station::{self, Placing};
use super::{Craft, Phase, MAX_STEPS, TABLE_RADIUS};
use crate::crafting::{
    plan, PlanStep, Stations, BREWING_STAND, FURNACE, SMITHING_TABLE, TABLE, WATER,
};
use crate::crafting::Recipe;
use crate::goals::task::GoalCx;

impl Craft {
    /// Plans from the inventory and sets up the first step.
    pub(super) fn replan(&mut self, cx: &mut GoalCx, wanted: u32) -> Result<(), String> {
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
        let stand = station::find(cx, BREWING_STAND);
        let smithing = station::find(cx, SMITHING_TABLE);
        // A stand already standing is taken to hold fuel until a brew at it came out
        // unbrewed; one this goal puts down is new, and the plan asked for its powder
        // while no stand stood near.
        // A blast furnace or a smoker near stands in for a furnace on the steps it takes.
        let quick = |cx: &GoalCx, r: &Recipe| {
            let stations = book.quick_stations(r);
            stations.into_iter().find_map(|s| station::find(cx, s))
        };
        let near = Stations {
            table: table.is_some(),
            furnace: furnace.is_some(),
            stand: stand.is_some(),
            smithing: smithing.is_some(),
            stand_fuelled: stand.is_some() && !self.stand_empty,
        };
        let inv = cx.world.inventory();
        let mut p = plan(&book, &inv, &self.item, wanted, near, self.smelt_only)?;
        if furnace.is_none() {
            // Planned as if a furnace stood near: holds when every furnace step has a
            // faster station to go to. Else the plan makes or asks for a furnace.
            let lent = Stations {
                furnace: true,
                ..near
            };
            if let Ok(q) = plan(&book, &inv, &self.item, wanted, lent, self.smelt_only) {
                let smelts = |s: &&PlanStep| matches!(s, PlanStep::Smelt { .. });
                if q.steps.iter().filter(smelts).all(|s| quick(cx, s.recipe()).is_some()) {
                    p = q;
                }
            }
        }
        // Water is found, never made: a plan that fills bottles ends here without any.
        let water = cx.world.water_source(cx.me_pos().block(), TABLE_RADIUS);
        let fills = |s: &PlanStep| s.recipe().station() == Some(WATER);
        if water.is_none() && p.steps.iter().any(fills) {
            let n: u32 = p.steps.iter().filter(|s| fills(s)).map(PlanStep::makes).sum();
            return Err(format!(
                "cannot craft {}: no water source within {TABLE_RADIUS} blocks to fill {n} \
                 glass_bottle at. Go to water, or carry a water_bucket and pour it out.",
                self.item
            ));
        }
        let steps: Vec<String> = p.steps.iter().map(PlanStep::describe).collect();
        cx.doing(format!("plan: {}", steps.join(", then ")));
        let held = |cx: &GoalCx, b: &str| cx.world.count_item(b) > 0;
        // The station a step is made at: the one standing near, else a held one to put down.
        let at = |cx: &GoalCx, found, name: &'static str, result: &str| match found {
            Some(p) => Ok(Ok(p)),
            None if held(cx, name) => Ok(Err(Phase::Place(Placing::new(name)))),
            None => Err(format!("{result} needs a {name}")),
        };
        let Some(step) = p.steps.into_iter().next() else {
            return Err(format!("nothing to do for {}", self.item));
        };
        self.made_by = step.recipe().verb();
        self.phase = match step {
            PlanStep::Craft { recipe, crafts } => {
                if cx.world.free_slots() == 0 {
                    return Err(format!("inventory full; no room for {}", recipe.result));
                }
                match recipe.station() {
                    None => Phase::Craft(recipe, crafts, None),
                    Some(BREWING_STAND) => match at(cx, stand, BREWING_STAND, &recipe.result)? {
                        Ok(p) => Phase::Brew(Brewing::new(p, recipe, crafts)),
                        Err(place) => place,
                    },
                    Some(WATER) => match water {
                        Some(w) => Phase::Fill(Filling::new(w, crafts)),
                        None => return Err(format!("{} needs water", recipe.result)),
                    },
                    Some(SMITHING_TABLE) => match at(cx, smithing, SMITHING_TABLE, &recipe.result)?
                    {
                        Ok(p) => Phase::Smith(Smithing::new(p, recipe)),
                        Err(place) => place,
                    },
                    Some(_) => match at(cx, table, TABLE, &recipe.result)? {
                        Ok(t) => Phase::Walk(t, recipe, crafts),
                        Err(place) => place,
                    },
                }
            }
            PlanStep::Smelt {
                recipe,
                count,
                fuel,
            } => match quick(cx, &recipe) {
                Some(f) => Phase::Smelt(Smelting::new(f, recipe, count, fuel).fast()),
                None => match at(cx, furnace, FURNACE, &recipe.result)? {
                    Ok(f) => Phase::Smelt(Smelting::new(f, recipe, count, fuel)),
                    Err(place) => place,
                },
            },
        };
        Ok(())
    }
}
