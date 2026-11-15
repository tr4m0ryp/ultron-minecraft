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
        }
    }

    pub fn missing(&self) -> u32 {
        match &self.via {
            Some(v) => v.subs.iter().map(Need::missing).sum(),
            None => self.need.saturating_sub(self.have),
        }
    }
}

/// When every alternative of a set is made the same way from one input (any plank from its
/// own logs), the shopping list names the union of those inputs ("any log"), not the first
/// alternative's ("oak_log").
pub fn widen(book: &RecipeBook, ing: &Ingredient, mut v: Via) -> Via {
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
