//! Recipes and the craft planner: the vanilla recipe book as the adapter loads it, the
//! dependency planner that turns (item, count) into ordered craft and smelt steps or one
//! shopping list, furnace fuel, and the concrete grid layout the adapter clicks in. Brewing
//! and smithing recipes are planned the same way: each names its station, and the goal
//! finds that station or puts a held one down.

mod book;
mod classify;
mod fuel;
mod label;
mod layout;
mod plan;

#[cfg(test)]
mod tests;

pub use book::{
    bottle_fill, Ingredient, Recipe, RecipeBook, Shape, BLAZE_POWDER, BREWING_STAND,
    BREWS_PER_POWDER, BREW_BOTTLES, FURNACE, SMITHING_TABLE, TABLE, WATER,
};
pub use fuel::{choose as choose_fuel, quarters as fuel_quarters};
pub use label::name_set;
pub use layout::{layout, matches, Layout};
pub use plan::{plan, CraftPlan, PlanStep, Stations, BUDGET};
