//! Registry names without the namespace, and query-kind matching over the registries.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use azalea::{
    block::BlockState,
    inventory::{components::PotionContents, ItemStack},
    registry::{builtin::BlockKind, Registry},
};
use reflex_core::goals::kind_matches;

pub fn bare(s: &'static str) -> &'static str {
    s.strip_prefix("minecraft:").unwrap_or(s)
}

pub fn block_name(state: BlockState) -> String {
    bare(BlockKind::from(state).to_str()).to_string()
}

/// The three bottle items whose contents make them different things to brew with.
const BOTTLES: [&str; 3] = ["potion", "splash_potion", "lingering_potion"];

/// A potion is named by what is in it: "water_potion", "healing_splash_potion". The recipe
/// loader names brewing inputs and outputs the same way, so the planner tells them apart.
pub fn potion_name(item: &str, potion: &str) -> String {
    format!("{potion}_{item}")
}

/// One leaked string per potion name ever seen (at most a few hundred), so every item name
/// stays `&'static str`.
fn intern(name: String) -> &'static str {
    static NAMES: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    let mut names = NAMES
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(n) = names.get(&name) {
        return n;
    }
    let leaked: &'static str = Box::leak(name.clone().into_boxed_str());
    names.insert(name, leaked);
    leaked
}

pub fn item_name(stack: &ItemStack) -> Option<&'static str> {
    if stack.is_empty() {
        return None;
    }
    let base = bare(stack.kind().to_str());
    if BOTTLES.contains(&base) {
        let contents = stack.get_component::<PotionContents>();
        if let Some(p) = contents.and_then(|c| c.potion) {
            return Some(intern(potion_name(base, bare(p.to_str()))));
        }
    }
    Some(base)
}

pub fn item_matches(kind: &str, stack: &ItemStack) -> bool {
    item_name(stack).is_some_and(|n| kind_matches(kind, n))
}

/// Every block kind whose name matches the query kind ("bed" gives all sixteen beds).
pub fn block_kinds(kind: &str) -> Vec<BlockKind> {
    (0u32..)
        .map_while(BlockKind::from_u32)
        .filter(|k| kind_matches(kind, bare(k.to_str())))
        .collect()
}
