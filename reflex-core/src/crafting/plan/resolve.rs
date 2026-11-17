//! The recursive search behind `plan`: a virtual inventory that ingredients are taken from,
//! recipes tried best-first (the share of their inputs already held), alternatives within a
//! tag tried until one works, and on failure the closest attempt kept for the shopping list.

use super::need::{weigh, widen, Need, Via, FETCHED};
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
        for (ing, want) in r.units(crafts) {
            held += self.held(ing).min(want);
            total += want;
        }
        (held * 1000).checked_div(total).unwrap_or(0)
    }

    /// `n` items of the set: held ones first, then the first alternative that can be made.
    pub fn need_set(&mut self, book: &RecipeBook, ing: &Ingredient, n: u32, mode: Mode) -> Need {
        let label = name_set(book, &ing.0);
        let have = self.take(ing, n);
        let rest = n - have;
        let leaf = |label: String| Need::leaf(label, n, have).weighing(weigh(book, &ing.0));
        if rest == 0 || self.budget == 0 || self.stack.len() >= MAX_DEPTH {
            return leaf(label);
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
            self.budget = trial.budget;
            if node.missing() == 0 {
                *self = trial;
                return Need {
                    label,
                    need: n,
                    have,
                    via: node.via,
                    weight: FETCHED,
                };
            }
            // The first alternative unless a later one is cheaper to come by.
            if primary.as_ref().is_none_or(|(_, p)| node.cost() < p.cost()) {
                primary = Some((trial, node));
            }
        }
        // A set holding a raw material is itself raw: list it, do not pick a recipe for it.
        let raw = ing.0.iter().any(|a| book.is_raw(a));
        match primary {
            Some((trial, node)) if !raw && node.via.is_some() => {
                *self = trial;
                let via = node.via.map(|v| widen(book, ing, v));
                Need {
                    label,
                    need: n,
                    have,
                    via,
                    weight: FETCHED,
                }
            }
            _ => leaf(label),
        }
    }

    /// Why an input a recipe takes only from the inventory is short: how the rest would
    /// be made, when it cannot be made from here either. Nothing is planned or taken.
    fn explain(
        &mut self,
        book: &RecipeBook,
        ing: &Ingredient,
        rest: u32,
        mode: Mode,
    ) -> Option<Via> {
        let mut trial = self.clone();
        let node = trial.need_set(book, ing, rest, mode);
        self.budget = trial.budget;
        node.via.filter(|_| node.missing() > 0)
    }

    /// `n` more of `item` (none taken from the inventory): its best recipe, or the closest
    /// miss when none works.
    pub fn make(&mut self, book: &RecipeBook, item: &str, n: u32, mode: Mode) -> Need {
        self.budget = self.budget.saturating_sub(1);
        let mut rs: Vec<&Recipe> = book
            .recipes_for(item)
            .iter()
            .filter(|r| mode.allows(r))
            .collect();
        let keys: Vec<u32> = rs.iter().map(|r| self.score(r, n)).collect();
        let mut order: Vec<usize> = (0..rs.len()).collect();
        order.sort_by_key(|i| std::cmp::Reverse(keys[*i]));
        rs = order.into_iter().map(|i| rs[i]).collect();
        let mut closest: Option<((u32, bool, u32), State, Need)> = None;
        for r in rs {
            let mut trial = self.clone();
            let node = trial.apply(book, r, n);
            self.budget = trial.budget;
            if node.missing() == 0 {
                *self = trial;
                return node;
            }
            // A recipe that takes its own result (copying a smithing template) is no way
            // to come by the first one.
            if r.inputs().iter().any(|(i, _)| i.accepts(&r.result)) {
                continue;
            }
            // Shopping advice names the route that is cheapest to come by, then one that
            // chains over one that needs its input held.
            let key = (node.cost(), book.in_hand_only(r), node.missing());
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
        let sub_mode = if r.is_smelt() {
            Mode::SmeltOnly
        } else {
            Mode::Any
        };
        self.stack.push(r.result.clone());
        let mut subs = Vec::new();
        let mut from = Vec::new();
        for (ing, want) in r.units(crafts) {
            from.push((name_set(book, &ing.0), want));
            subs.push(if in_hand {
                let have = self.take(ing, want);
                let mut leaf =
                    Need::leaf(name_set(book, &ing.0), want, have).weighing(weigh(book, &ing.0));
                if have < want {
                    leaf.via = self.explain(book, ing, want - have, sub_mode);
                }
                leaf
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
            verb: r.verb(),
            makes: crafts * r.count,
            from,
            subs,
        };
        Need {
            label: r.result.clone(),
            need: n,
            have: 0,
            via: Some(via),
            weight: FETCHED,
        }
    }
}
