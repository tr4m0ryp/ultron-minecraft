//! The recursive search behind `plan`: a virtual inventory that ingredients are taken from,
//! recipes tried best-first (the share of their inputs already held), alternatives within a
//! tag tried until one works, and on failure the closest attempt kept for the shopping list.

use super::{PlanStep, BUDGET};
use crate::crafting::book::{Ingredient, Recipe, RecipeBook};
use crate::crafting::label::name_set;

/// Chains longer than this are cut (vanilla's deepest useful chain is about six).
const MAX_DEPTH: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Any,
    /// Furnace recipes only: a smelt's input is held or itself smelted, never crafted.
    SmeltOnly,
}

impl Mode {
    pub fn allows(self, r: &Recipe) -> bool {
        self == Mode::Any || r.is_smelt()
    }
}

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
    pub smelt: bool,
    pub makes: u32,
    /// The inputs of all the crafts together, as (label, units).
    pub from: Vec<(String, u32)>,
    pub subs: Vec<Need>,
}

impl Need {
    fn leaf(label: String, need: u32, have: u32) -> Self {
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

#[derive(Clone, Debug)]
pub struct State {
    pub inv: Vec<(String, u32)>,
    pub steps: Vec<PlanStep>,
    budget: u32,
    stack: Vec<String>,
}

impl State {
    pub fn new(inv: &[(String, u32)]) -> Self {
        Self {
            inv: inv.iter().filter(|(_, c)| *c > 0).cloned().collect(),
            steps: Vec::new(),
            budget: BUDGET,
            stack: Vec::new(),
        }
    }

    fn held(&self, ing: &Ingredient) -> u32 {
        self.inv
            .iter()
            .filter(|(n, _)| ing.accepts(n))
            .map(|(_, c)| c)
            .sum()
    }

    /// Takes up to `n` items the ingredient accepts, largest stacks first.
    fn take(&mut self, ing: &Ingredient, n: u32) -> u32 {
        let mut left = n;
        let mut order: Vec<usize> = (0..self.inv.len())
            .filter(|i| ing.accepts(&self.inv[*i].0))
            .collect();
        order.sort_by_key(|i| std::cmp::Reverse(self.inv[*i].1));
        for i in order {
            let t = self.inv[i].1.min(left);
            self.inv[i].1 -= t;
            left -= t;
        }
        self.inv.retain(|(_, c)| *c > 0);
        n - left
    }

    pub fn take_exact(&mut self, item: &str, n: u32) -> u32 {
        self.take(&Ingredient::one(item), n)
    }

    fn give(&mut self, item: &str, n: u32) {
        if n == 0 {
            return;
        }
        match self.inv.iter_mut().find(|(k, _)| k == item) {
            Some((_, c)) => *c += n,
            None => self.inv.push((item.to_string(), n)),
        }
    }

    /// Per mille of the inputs of `crafts` crafts the inventory already holds.
    fn score(&self, r: &Recipe, wanted: u32) -> u32 {
        let crafts = wanted.div_ceil(r.count.max(1));
        let (mut held, mut total) = (0, 0);
        for (ing, units) in r.inputs() {
            held += self.held(ing).min(units * crafts);
            total += units * crafts;
        }
        (held * 1000).checked_div(total).unwrap_or(0)
    }

    /// `n` items of the set: held ones first, then the first alternative that can be made.
    pub fn need_set(&mut self, book: &RecipeBook, ing: &Ingredient, n: u32, mode: Mode) -> Need {
        let label = name_set(book, &ing.0);
        let have = self.take(ing, n);
        let rest = n - have;
        if rest == 0 || self.budget == 0 || self.stack.len() >= MAX_DEPTH {
            return Need::leaf(label, n, have);
        }
        let usable = |a: &String| {
            !self.stack.contains(a) && book.recipes_for(a).iter().any(|r| mode.allows(r))
        };
        let mut alts: Vec<&String> = ing.0.iter().filter(|a| usable(a)).collect();
        let best = |a: &String| {
            let rs = book.recipes_for(a).iter().filter(|r| mode.allows(r));
            rs.map(|r| self.score(r, rest)).max().unwrap_or(0)
        };
        let keys: Vec<u32> = alts.iter().map(|a| best(a)).collect();
        let mut order: Vec<usize> = (0..alts.len()).collect();
        order.sort_by_key(|i| std::cmp::Reverse(keys[*i]));
        alts = order.into_iter().map(|i| alts[i]).collect();
        let mut primary: Option<(State, Need)> = None;
        for a in &alts {
            let mut trial = self.clone();
            let node = trial.make(book, a, rest, mode);
            if node.missing() == 0 {
                *self = trial;
                return Need { label, need: n, have, via: node.via };
            }
            primary.get_or_insert((trial, node));
        }
        // A set holding a raw material is itself raw: list it, do not pick a recipe for it.
        let raw = ing.0.iter().any(|a| !book.has_recipe(a));
        match primary {
            Some((trial, node)) if !raw => {
                *self = trial;
                let via = node.via.map(|v| widen(book, ing, v));
                Need { label, need: n, have, via }
            }
            _ => Need::leaf(label, n, have),
        }
    }

    /// `n` more of `item` (none taken from the inventory): its best recipe, or the closest
    /// miss when none works.
    pub fn make(&mut self, book: &RecipeBook, item: &str, n: u32, mode: Mode) -> Need {
        self.budget = self.budget.saturating_sub(1);
        let mut rs: Vec<&Recipe> = book.recipes_for(item).iter().filter(|r| mode.allows(r)).collect();
        let keys: Vec<u32> = rs.iter().map(|r| self.score(r, n)).collect();
        let mut order: Vec<usize> = (0..rs.len()).collect();
        order.sort_by_key(|i| std::cmp::Reverse(keys[*i]));
        rs = order.into_iter().map(|i| rs[i]).collect();
        let mut closest: Option<((bool, u32), State, Need)> = None;
        for r in rs {
            let mut trial = self.clone();
            let node = trial.apply(book, r, n);
            if node.missing() == 0 {
                *self = trial;
                return node;
            }
            // Shopping advice prefers a recipe that chains over one that needs its input held.
            let key = (book.in_hand_only(r), node.missing());
            if closest.as_ref().is_none_or(|(k, _, _)| key < *k) {
                closest = Some((key, trial, node));
            }
        }
        match closest {
            Some((_, trial, node)) => {
                *self = trial;
                node
            }
            None => Need::leaf(item.to_string(), n, 0),
        }
    }

    /// Runs `r` enough times for `n` items; the surplus goes back into the inventory.
    fn apply(&mut self, book: &RecipeBook, r: &Recipe, n: u32) -> Need {
        let crafts = n.div_ceil(r.count.max(1));
        let in_hand = book.in_hand_only(r);
        let sub_mode = if r.is_smelt() { Mode::SmeltOnly } else { Mode::Any };
        self.stack.push(r.result.clone());
        let mut subs = Vec::new();
        let mut from = Vec::new();
        for (ing, units) in r.inputs() {
            let want = units * crafts;
            from.push((name_set(book, &ing.0), want));
            subs.push(if in_hand {
                let have = self.take(ing, want);
                Need::leaf(name_set(book, &ing.0), want, have)
            } else {
                self.need_set(book, ing, want, sub_mode)
            });
        }
        self.stack.pop();
        self.steps.push(match r.is_smelt() {
            true => PlanStep::Smelt {
                recipe: r.clone(),
                count: crafts,
                fuel: Vec::new(),
            },
            false => PlanStep::Craft {
                recipe: r.clone(),
                crafts,
            },
        });
        self.give(&r.result, crafts * r.count - n);
        let via = Via {
            smelt: r.is_smelt(),
            makes: crafts * r.count,
            from,
            subs,
        };
        Need { label: r.result.clone(), need: n, have: 0, via: Some(via) }
    }
}

/// When every alternative of a set is made the same way from one input (any plank from its
/// own logs), the shopping list names the union of those inputs ("any log"), not the first
/// alternative's ("oak_log").
fn widen(book: &RecipeBook, ing: &Ingredient, mut v: Via) -> Via {
    let mut union: Vec<String> = Vec::new();
    for a in &ing.0 {
        let Some(r) = book.recipes_for(a).iter().find(|r| !r.is_smelt()) else {
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
