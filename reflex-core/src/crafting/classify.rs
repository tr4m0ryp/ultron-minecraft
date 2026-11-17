//! Recipes the planner uses only with their inputs already in hand. Chaining them invites
//! cycles (iron_ingot -> iron_block -> iron_ingot) or absurd detours (dyeing red wool white
//! to make a bed, melting a crafted sword for a nugget), so they never pull further crafting
//! in behind them.

use std::collections::HashSet;

use super::book::{Recipe, RecipeBook, Shape};

/// The last `_` word of a name: "wool" for "red_wool".
fn family(item: &str) -> &str {
    item.rsplit('_').next().unwrap_or(item)
}

/// Ids of the recipes that run only from the inventory.
pub fn in_hand_only(book: &RecipeBook) -> HashSet<String> {
    book.all()
        .filter(|r| self_referential(r) || recolour(r) || reversible(book, r) || recycle(book, r))
        .map(|r| r.id.clone())
        .collect()
}

/// An input accepts the result itself.
fn self_referential(r: &Recipe) -> bool {
    r.inputs().iter().any(|(i, _)| i.accepts(&r.result))
}

/// An input is a choice among items of the result's own family (any wool for white wool).
fn recolour(r: &Recipe) -> bool {
    let fam = family(&r.result);
    r.inputs()
        .iter()
        .any(|(i, _)| i.0.len() > 1 && i.0.iter().all(|a| family(a) == fam))
}

/// A one-input craft whose input can be crafted back from the result: storage blocks and
/// nuggets in both directions.
fn reversible(book: &RecipeBook, r: &Recipe) -> bool {
    if r.verb() != "craft" {
        return false;
    }
    let inputs = r.inputs();
    let [(only, _)] = inputs.as_slice() else {
        return false;
    };
    only.0.iter().any(|x| {
        book.recipes_for(x)
            .iter()
            .filter(|back| !back.is_smelt())
            .any(|back| back.inputs().iter().any(|(i, _)| i.accepts(&r.result)))
    })
}

/// A smelt that melts crafted goods back down (iron tools into nuggets). Most of its inputs
/// are crafted, not all: the vanilla list also holds horse armour, which is only found, and
/// with "all" the gold one counted as a way to make nuggets: the shopping list for a
/// glistering melon asked for 8 golden pickaxes.
fn recycle(book: &RecipeBook, r: &Recipe) -> bool {
    match &r.shape {
        Shape::Smelt { input, .. } => {
            let made = input.0.iter().filter(|i| book.has_recipe(i)).count();
            input.0.len() > 1 && made * 2 > input.0.len()
        }
        _ => false,
    }
}

/// Items that are fetched, not made, though recipes give them: what an ore drops (coal,
/// diamond, raw gold) or a crop. Every recipe for such an item unpacks its storage block
/// (one block into several) or smelts the ore block itself, which a pickaxe never gives.
/// Ingots are not among them: an ingot is also smelted from raw metal.
pub fn fetched(book: &RecipeBook) -> HashSet<String> {
    let unpack = |r: &Recipe| r.count > 1 && reversible(book, r);
    let ore = |r: &Recipe| match &r.shape {
        Shape::Smelt { input, .. } => input.0.iter().all(|i| i.ends_with("_ore")),
        _ => false,
    };
    let mut out: HashSet<String> = book.all().map(|r| r.result.clone()).collect();
    out.retain(|item| book.recipes_for(item).iter().all(|r| unpack(r) || ore(r)));
    out
}
