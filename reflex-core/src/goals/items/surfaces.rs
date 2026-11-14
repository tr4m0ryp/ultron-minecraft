//! How a placement sees the blocks around its target: which it may overwrite and which
//! would open a menu instead of taking a block when clicked.

/// Blocks a placement may overwrite: the server's `#replaceable` tag (plants, litter,
/// thin layers and fluids that a placed block simply takes the place of).
pub fn replaceable(name: &str) -> bool {
    matches!(
        name,
        "air"
            | "cave_air"
            | "void_air"
            | "water"
            | "lava"
            | "short_grass"
            | "tall_grass"
            | "fern"
            | "large_fern"
            | "bush"
            | "dead_bush"
            | "short_dry_grass"
            | "tall_dry_grass"
            | "leaf_litter"
            | "snow"
            | "vine"
            | "glow_lichen"
            | "resin_clump"
            | "seagrass"
            | "tall_seagrass"
            | "fire"
            | "soul_fire"
            | "crimson_roots"
            | "warped_roots"
            | "nether_sprouts"
            | "hanging_roots"
            | "light"
            | "structure_void"
    )
}

/// Blocks whose right click opens a menu or toggles them, so a click on them never places.
pub fn opens_on_click(name: &str) -> bool {
    const EXACT: [&str; 24] = [
        "crafting_table",
        "furnace",
        "smoker",
        "blast_furnace",
        "barrel",
        "loom",
        "stonecutter",
        "grindstone",
        "cartography_table",
        "smithing_table",
        "enchanting_table",
        "brewing_stand",
        "beacon",
        "hopper",
        "dispenser",
        "dropper",
        "crafter",
        "note_block",
        "repeater",
        "comparator",
        "lever",
        "bell",
        "lectern",
        "respawn_anchor",
    ];
    const SUFFIX: [&str; 8] = [
        "chest",
        "shulker_box",
        "_bed",
        "_door",
        "_trapdoor",
        "_fence_gate",
        "_button",
        "anvil",
    ];
    EXACT.contains(&name) || SUFFIX.iter().any(|s| name.ends_with(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menus_and_toggles_are_not_supports() {
        for b in [
            "crafting_table",
            "chest",
            "trapped_chest",
            "oak_door",
            "red_bed",
            "chipped_anvil",
        ] {
            assert!(opens_on_click(b), "{b}");
        }
        for b in ["stone", "oak_planks", "dirt", "oak_log", "cobblestone"] {
            assert!(!opens_on_click(b), "{b}");
        }
    }
}
