//! Registry names without the namespace, and query-kind matching over the registries.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use azalea::{
    block::BlockState,
    inventory::{components::PotionContents, ItemStack},
    registry::{
        builtin::{BlockKind, ItemKind, Potion},
        Registry,
    },
};
use reflex_core::goals::name_matches;

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

/// Every name a block or an item goes by here: both registries, and each bottle by each
/// of its contents.
fn names() -> &'static HashSet<String> {
    static NAMES: OnceLock<HashSet<String>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let blocks = (0u32..)
            .map_while(BlockKind::from_u32)
            .map(|k| bare(k.to_str()).to_string());
        let items = (0u32..)
            .map_while(ItemKind::from_u32)
            .map(|k| bare(k.to_str()).to_string());
        let potions = (0u32..).map_while(Potion::from_u32).flat_map(|p| {
            BOTTLES
                .iter()
                .map(move |b| potion_name(b, bare(p.to_str())))
        });
        blocks.chain(items).chain(potions).collect()
    })
}

/// `kind` is itself the name of a block or an item, not a family word like "bed".
pub fn full_name(kind: &str) -> bool {
    names().contains(kind)
}

/// Whether `name` is what the query `kind` asks for: the name itself when `kind` is one,
/// else every name of the family (`reflex_core::goals::name_matches`).
pub fn matches(kind: &str, name: &str) -> bool {
    name_matches(kind, name, full_name(kind))
}

pub fn item_matches(kind: &str, stack: &ItemStack) -> bool {
    item_name(stack).is_some_and(|n| matches(kind, n))
}

/// Every block kind the query kind asks for ("bed" gives every bed, "furnace" only the
/// furnace).
pub fn block_kinds(kind: &str) -> Vec<BlockKind> {
    let full = full_name(kind);
    (0u32..)
        .map_while(BlockKind::from_u32)
        .filter(|k| name_matches(kind, bare(k.to_str()), full))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_name_matches_only_itself() {
        assert!(matches("healing_potion", "healing_potion"));
        assert!(!matches("healing_potion", "strong_healing_potion"));
        assert!(!matches("healing_potion", "long_healing_potion"));
        assert!(!matches("furnace", "blast_furnace"));
        assert!(!matches("beef", "cooked_beef"));
        assert!(!matches("stone", "smooth_stone"));
        assert!(!matches("dirt", "coarse_dirt"));
        assert!(!matches("bucket", "water_bucket"));
        assert!(!matches("melon_slice", "glistering_melon_slice"));
    }

    #[test]
    fn a_family_word_and_an_ore_keep_their_variants() {
        assert!(!full_name("bed") && matches("bed", "red_bed"));
        assert!(matches("log", "oak_log") && matches("planks", "birch_planks"));
        assert!(matches("pickaxe", "iron_pickaxe") && matches("wool", "white_wool"));
        assert!(matches("iron_ore", "deepslate_iron_ore"));
        assert!(!matches("gold_ore", "nether_gold_ore"));
        let furnaces: Vec<_> = block_kinds("furnace")
            .iter()
            .map(|k| bare(k.to_str()))
            .collect();
        assert_eq!(furnaces, ["furnace"]);
        let beds = block_kinds("bed");
        assert!(beds.len() >= 16 && beds.iter().all(|k| k.to_str().ends_with("_bed")));
    }
}
