//! Recipe data as the planner sees it: concrete item names (no namespace), tags already
//! expanded into the items they hold, crafting and smelting recipes keyed by result.

use std::collections::{HashMap, HashSet};

use super::classify;

/// The items one grid cell or furnace slot accepts (a tag or an ingredient list, expanded).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Ingredient(pub Vec<String>);

impl Ingredient {
    pub fn one(item: &str) -> Self {
        Self(vec![item.to_string()])
    }
    pub fn of(items: &[&str]) -> Self {
        Self(items.iter().map(|s| s.to_string()).collect())
    }
    pub fn accepts(&self, item: &str) -> bool {
        self.0.iter().any(|i| i == item)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// Row-major `width` x `height` pattern; `None` is an empty cell. The server also
    /// accepts the pattern mirrored left to right.
    Shaped {
        width: u8,
        height: u8,
        cells: Vec<Option<Ingredient>>,
    },
    /// Every ingredient once, in any cells.
    Shapeless(Vec<Ingredient>),
    /// One input per output in a furnace, `ticks` of cooking each.
    Smelt { input: Ingredient, ticks: u32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Recipe {
    /// The data file's name, e.g. "stick_from_bamboo_item".
    pub id: String,
    pub result: String,
    /// Items one craft (or one smelt) produces.
    pub count: u32,
    pub shape: Shape,
}

impl Recipe {
    pub fn is_smelt(&self) -> bool {
        matches!(self.shape, Shape::Smelt { .. })
    }

    /// A 3x3 grid, so a crafting table.
    pub fn needs_table(&self) -> bool {
        match &self.shape {
            Shape::Shaped { width, height, .. } => *width > 2 || *height > 2,
            Shape::Shapeless(v) => v.len() > 4,
            Shape::Smelt { .. } => false,
        }
    }

    /// Distinct ingredients with the units one craft consumes, in first-use order.
    pub fn inputs(&self) -> Vec<(&Ingredient, u32)> {
        let all: Vec<&Ingredient> = match &self.shape {
            Shape::Shaped { cells, .. } => cells.iter().flatten().collect(),
            Shape::Shapeless(v) => v.iter().collect(),
            Shape::Smelt { input, .. } => vec![input],
        };
        let mut out: Vec<(&Ingredient, u32)> = Vec::new();
        for ing in all {
            match out.iter_mut().find(|(i, _)| *i == ing) {
                Some((_, n)) => *n += 1,
                None => out.push((ing, 1)),
            }
        }
        out
    }

    /// Grid cells (row-major index in a `grid`-wide grid) and the ingredient each takes.
    pub fn cells(&self, grid: usize) -> Vec<(usize, &Ingredient)> {
        match &self.shape {
            Shape::Shaped { width, cells, .. } => cells
                .iter()
                .enumerate()
                .filter_map(|(i, c)| {
                    let (row, col) = (i / *width as usize, i % *width as usize);
                    c.as_ref().map(|c| (row * grid + col, c))
                })
                .collect(),
            Shape::Shapeless(v) => v.iter().enumerate().collect(),
            Shape::Smelt { .. } => Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RecipeBook {
    by_result: HashMap<String, Vec<Recipe>>,
    tags: Vec<(String, Vec<String>)>,
    in_hand: HashSet<String>,
}

impl RecipeBook {
    pub fn add(&mut self, r: Recipe) {
        self.by_result.entry(r.result.clone()).or_default().push(r);
    }

    /// A resolved item tag ("planks" with its items), used to name sets of items.
    pub fn add_tag(&mut self, name: &str, items: Vec<String>) {
        self.tags.push((name.to_string(), items));
    }

    /// Orders each item's recipes (the one named after the item first, crafting before
    /// smelting) and marks the recipes that are only used with their inputs in hand.
    pub fn finish(&mut self) {
        for (item, rs) in self.by_result.iter_mut() {
            rs.sort_by_key(|r| (r.is_smelt(), r.id != *item, r.id.len(), r.id.clone()));
        }
        self.tags.sort_by(|a, b| a.0.cmp(&b.0));
        self.in_hand = classify::in_hand_only(self);
    }

    pub fn recipes_for(&self, item: &str) -> &[Recipe] {
        self.by_result.get(item).map_or(&[], Vec::as_slice)
    }

    pub fn has_recipe(&self, item: &str) -> bool {
        self.by_result.contains_key(item)
    }

    pub fn all(&self) -> impl Iterator<Item = &Recipe> {
        self.by_result.values().flatten()
    }

    pub fn len(&self) -> usize {
        self.by_result.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.by_result.is_empty()
    }

    pub fn tags(&self) -> &[(String, Vec<String>)] {
        &self.tags
    }

    /// Storage-block packing and unpacking, nuggets and recolouring: such a recipe runs
    /// only when its inputs are already in the inventory, never as a link in a chain.
    pub fn in_hand_only(&self, r: &Recipe) -> bool {
        self.in_hand.contains(&r.id)
    }
}
