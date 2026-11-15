//! The craft planner: resolves the recipe tree for (item, count) against the inventory and
//! returns the ordered craft and smelt steps, or one shopping list naming exactly what is
//! missing. Pure logic over a `RecipeBook`; the goal re-plans after every executed step.

mod need;
mod resolve;
mod text;

use super::book::{
    Recipe, RecipeBook, BLAZE_POWDER, BREWING_STAND, BREWS_PER_POWDER, BREW_BOTTLES, FURNACE,
    SMITHING_TABLE, TABLE,
};
use super::fuel;
use resolve::{Mode, State};

/// Make calls the search may spend on one plan before it gives up on further alternatives.
pub const BUDGET: u32 = 20_000;

#[derive(Clone, Debug, PartialEq)]
pub enum PlanStep {
    /// A grid craft, or `crafts` bottles at a brewing stand, or `crafts` items at a
    /// smithing table: the recipe's shape says which.
    Craft {
        recipe: Recipe,
        crafts: u32,
    },
    /// Smelt `count` items with `recipe`, burning `fuel` (empty until the plan is complete).
    Smelt {
        recipe: Recipe,
        count: u32,
        fuel: Vec<(String, u32)>,
    },
}

impl PlanStep {
    pub fn recipe(&self) -> &Recipe {
        match self {
            PlanStep::Craft { recipe, .. } | PlanStep::Smelt { recipe, .. } => recipe,
        }
    }

    pub fn makes(&self) -> u32 {
        match self {
            PlanStep::Craft { recipe, crafts } => recipe.count * crafts,
            PlanStep::Smelt { recipe, count, .. } => recipe.count * count,
        }
    }

    /// "craft 4 stick", "smelt 3 iron_ingot", "brew 1 awkward_potion".
    pub fn describe(&self) -> String {
        let r = self.recipe();
        format!("{} {} {}", r.verb(), self.makes(), r.result)
    }
}

/// Stations already standing within reach of a walk.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stations {
    pub table: bool,
    pub furnace: bool,
    pub stand: bool,
    pub smithing: bool,
    /// The stand near is known to hold blaze fuel (this goal loaded it).
    pub stand_fuelled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CraftPlan {
    pub steps: Vec<PlanStep>,
    /// Some step needs a crafting table (one stands near, is held, or is crafted first).
    pub needs_table: bool,
    pub needs_furnace: bool,
}

/// Plans `count` new `item`s (held ones do not count toward it) from the inventory `inv`.
/// `smelt_only` restricts the last step to a furnace. The error is one line for thought:
/// "no recipe ..." or the shopping list.
pub fn plan(
    book: &RecipeBook,
    inv: &[(String, u32)],
    item: &str,
    count: u32,
    near: Stations,
    smelt_only: bool,
) -> Result<CraftPlan, String> {
    let mode = if smelt_only {
        Mode::SmeltOnly
    } else {
        Mode::Any
    };
    let verb = if smelt_only { "smelt" } else { "craft" };
    if !book.recipes_for(item).iter().any(|r| mode.allows(r)) {
        let how = if smelt_only {
            "smelting recipe"
        } else {
            "recipe"
        };
        return Err(format!("no {how} makes {item}"));
    }
    // Things the plan turned out to need beside the item: stations not standing near, and
    // blaze powder for an unfuelled stand. Each is taken from the inventory, or made, before
    // the item itself, so the plan never spends them on something else.
    let mut extras: Vec<(&str, u32)> = Vec::new();
    loop {
        let mut st = State::new(inv);
        let mut nodes: Vec<need::Need> = extras
            .iter()
            .map(|(s, n)| st.need_set(book, &super::Ingredient::one(s), *n, Mode::Any))
            .collect();
        let top = st.make(book, item, count.max(1), mode);
        let at = |name: &str| st.steps.iter().any(|s| s.recipe().station() == Some(name));
        let (table, furnace) = (at(TABLE), at(FURNACE));
        let lacking = [
            (FURNACE, near.furnace),
            (BREWING_STAND, near.stand),
            (SMITHING_TABLE, near.smithing),
        ]
        .into_iter()
        .find(|(name, near)| at(name) && !near && !extras.iter().any(|(e, _)| e == name));
        if let Some((name, _)) = lacking {
            extras.push((name, 1));
            continue;
        }
        if table && !near.table && !extras.iter().any(|(e, _)| *e == TABLE) {
            extras.insert(0, (TABLE, 1));
            continue;
        }
        let batches: u32 = st.steps.iter().map(brew_batches).sum();
        if batches > 0 && !near.stand_fuelled && !extras.iter().any(|(e, _)| *e == BLAZE_POWDER) {
            extras.push((BLAZE_POWDER, batches.div_ceil(BREWS_PER_POWDER)));
            continue;
        }
        let short_fuel = add_fuel(&mut st);
        let missing: u32 = nodes.iter().map(|n| n.missing()).sum::<u32>() + top.missing();
        if missing == 0 && short_fuel == 0 {
            return Ok(CraftPlan {
                steps: st.steps,
                needs_table: table,
                needs_furnace: furnace,
            });
        }
        nodes.extend(top.via.map(|v| v.subs).unwrap_or_default());
        return Err(text::shopping(verb, item, &nodes, short_fuel));
    }
}

/// Batches a step runs at a brewing stand (0 for any other step).
fn brew_batches(step: &PlanStep) -> u32 {
    match step {
        PlanStep::Craft { recipe, crafts } if recipe.station() == Some(BREWING_STAND) => {
            crafts.div_ceil(BREW_BOTTLES)
        }
        _ => 0,
    }
}

/// Fuel for each smelt step from what the plan leaves over; returns the smelts uncovered.
fn add_fuel(st: &mut State) -> u32 {
    let mut short = 0;
    for i in 0..st.steps.len() {
        let PlanStep::Smelt { count, .. } = st.steps[i] else {
            continue;
        };
        let (picks, left) = fuel::choose(&st.inv, count);
        for (name, n) in &picks {
            st.take_exact(name, *n);
        }
        short += left.div_ceil(4);
        if let PlanStep::Smelt { fuel, .. } = &mut st.steps[i] {
            *fuel = picks;
        }
    }
    short
}
