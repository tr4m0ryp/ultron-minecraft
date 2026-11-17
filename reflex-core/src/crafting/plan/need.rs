//! The planner's tree of needs: what each link wants, holds and is made from, and the
//! widening of a failed set's input to the union its alternatives accept.

use crate::crafting::book::{Ingredient, RecipeBook};
use crate::crafting::label::name_set;

/// One line of the tree: `need` of `label`, `have` taken from the inventory, and how the
/// rest was (or would be) made.
#[derive(Clone, Debug, PartialEq)]
pub struct Need {
    pub label: String,
    pub need: u32,
    pub have: u32,
    pub via: Option<Via>,
    /// What one missing unit costs to come by when nothing makes it here (see `weigh`).
    pub weight: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Via {
    /// "craft", "smelt", "brew" or "smith".
    pub verb: &'static str,
    pub makes: u32,
    /// The inputs of all the crafts together, as (label, units).
    pub from: Vec<(String, u32)>,
    pub subs: Vec<Need>,
}

impl Need {
    pub fn leaf(label: String, need: u32, have: u32) -> Self {
        Self {
            label,
            need,
            have,
            via: None,
            weight: FETCHED,
        }
    }

    pub fn weighing(mut self, weight: u32) -> Self {
        self.weight = weight;
        self
    }

    /// The effort the missing leaves stand for: the route with the least of it is the
    /// one the shopping list names.
    pub fn cost(&self) -> u32 {
        match &self.via {
            Some(v) => v.subs.iter().map(Need::cost).sum(),
            None => self.need.saturating_sub(self.have) * self.weight,
        }
    }

    pub fn missing(&self) -> u32 {
        match &self.via {
            Some(v) => v.subs.iter().map(Need::missing).sum(),
            None => self.need.saturating_sub(self.have),
        }
    }
}

/// One unit of something mined, farmed or found.
pub const FETCHED: u32 = 1;
/// An ore block as an item needs silk touch; what the ore drops is the usual way.
const ORE_BLOCK: u32 = 2;
/// A crafted thing the chain stops at (a tool to melt down, a link cut for a cycle).
const MADE: u32 = 8;

/// The cost of one missing unit of a set of acceptable items. Live, the list for a
/// glistering melon read "smelt 8 gold_nugget from 8 one of 12 kinds such as
/// golden_pickaxe" where one gold ingot, from one raw gold, would do.
pub fn weigh(book: &RecipeBook, items: &[String]) -> u32 {
    let made = items.iter().filter(|i| !book.is_raw(i)).count();
    if made * 2 > items.len() {
        MADE
    } else if items.iter().all(|i| i.ends_with("_ore")) {
        ORE_BLOCK
    } else {
        FETCHED
    }
}

/// When every alternative of a set is made the same way from one input (any plank from its
/// own logs), the shopping list names the union of those inputs ("any log"), not the first
/// alternative's ("oak_log").
pub fn widen(book: &RecipeBook, ing: &Ingredient, mut v: Via) -> Via {
    // Only a craft is widened: live, a smelt kept its verb and got the craft's input, and
    // the list read "smelt 4 gold_ingot from 4 gold_nugget".
    if v.verb != "craft" {
        return v;
    }
    let mut union: Vec<String> = Vec::new();
    for a in &ing.0 {
        let Some(r) = book.recipes_for(a).iter().find(|r| r.verb() == "craft") else {
            return v;
        };
        let inputs = r.inputs();
        let [(only, _)] = inputs.as_slice() else {
            return v;
        };
        for i in &only.0 {
            if !union.contains(i) {
                union.push(i.clone());
            }
        }
    }
    if let ([(label, _)], [sub]) = (v.from.as_mut_slice(), v.subs.as_mut_slice()) {
        let wide = name_set(book, &union);
        *label = wide.clone();
        sub.label = wide;
    }
    v
}
