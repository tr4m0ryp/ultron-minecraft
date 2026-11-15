//! The craft planner: resolves the recipe tree for (item, count) against the inventory and
//! returns the ordered craft and smelt steps, or one shopping list naming exactly what is
//! missing. Pure logic over a `RecipeBook`; the goal re-plans after every executed step.

mod need;
mod resolve;
mod text;

use super::book::{Recipe, RecipeBook};
use super::fuel;
use resolve::{Mode, State};

/// Make calls the search may spend on one plan before it gives up on further alternatives.
pub const BUDGET: u32 = 20_000;
pub const TABLE: &str = "crafting_table";
pub const FURNACE: &str = "furnace";

#[derive(Clone, Debug, PartialEq)]
pub enum PlanStep {
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

    /// "craft 4 stick", "smelt 3 iron_ingot".
    pub fn describe(&self) -> String {
        let verb = match self {
            PlanStep::Craft { .. } => "craft",
            PlanStep::Smelt { .. } => "smelt",
        };
        format!("{verb} {} {}", self.makes(), self.recipe().result)
    }
}

/// Stations already standing within reach of a walk.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stations {
    pub table: bool,
    pub furnace: bool,
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
    let mut stations: Vec<&str> = Vec::new();
    loop {
        let mut st = State::new(inv);
        let mut nodes: Vec<need::Need> = stations
            .iter()
            .map(|s| st.need_set(book, &super::Ingredient::one(s), 1, Mode::Any))
            .collect();
        let top = st.make(book, item, count.max(1), mode);
        let table = st.steps.iter().any(|s| s.recipe().needs_table());
        let furnace = st.steps.iter().any(|s| matches!(s, PlanStep::Smelt { .. }));
        if furnace && !near.furnace && !stations.contains(&FURNACE) {
            stations.push(FURNACE);
            continue;
        }
        if table && !near.table && !stations.contains(&TABLE) {
            stations.insert(0, TABLE);
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
