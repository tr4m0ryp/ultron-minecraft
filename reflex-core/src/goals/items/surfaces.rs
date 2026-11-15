//! How a placement sees the blocks around its target: which it may overwrite and which
//! would open a menu instead of taking a block when clicked.

use crate::math::{BlockPos, Vec3};

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

/// Natural blocks a placement may dig out of its way first: leaves, loose and soft ground,
/// snow and flora. Stone, ores and anything a player builds with stay: a placement into
/// those fails unless the goal says `replace`.
pub fn soft_natural(name: &str) -> bool {
    const EXACT: [&str; 36] = [
        "dirt",
        "coarse_dirt",
        "rooted_dirt",
        "grass_block",
        "podzol",
        "mycelium",
        "dirt_path",
        "farmland",
        "mud",
        "clay",
        "sand",
        "red_sand",
        "gravel",
        "snow_block",
        "moss_block",
        "moss_carpet",
        "pale_moss_block",
        "pale_moss_carpet",
        "dandelion",
        "poppy",
        "blue_orchid",
        "allium",
        "azure_bluet",
        "oxeye_daisy",
        "cornflower",
        "lily_of_the_valley",
        "torchflower",
        "sunflower",
        "lilac",
        "rose_bush",
        "peony",
        "pink_petals",
        "wildflowers",
        "sweet_berry_bush",
        "brown_mushroom",
        "red_mushroom",
    ];
    EXACT.contains(&name) || name.ends_with("_leaves") || name.ends_with("_tulip")
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

/// Whether an eye at `eye` sees the face `support` shares with `target`: it must sit on the
/// target's side of that face's plane.
pub fn face_visible(eye: Vec3, support: BlockPos, target: BlockPos) -> bool {
    const MARGIN: f64 = 0.05;
    let d = (
        target.x - support.x,
        target.y - support.y,
        target.z - support.z,
    );
    let (e, plane, sign) = match d {
        (1, 0, 0) | (-1, 0, 0) => (eye.x, f64::from(support.x.max(target.x)), d.0),
        (0, 1, 0) | (0, -1, 0) => (eye.y, f64::from(support.y.max(target.y)), d.1),
        (0, 0, 1) | (0, 0, -1) => (eye.z, f64::from(support.z.max(target.z)), d.2),
        _ => return false,
    };
    (e - plane) * f64::from(sign) > MARGIN
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wall_top_is_hidden_from_below_and_seen_from_above() {
        let wall = BlockPos::new(8, 72, 7);
        let roof = wall.above();
        let inside_eye = Vec3::new(8.5, 71.0 + 1.62, 5.5);
        assert!(!face_visible(inside_eye, wall, roof));
        assert!(face_visible(Vec3::new(8.5, 72.0 + 1.62, 5.5), wall, roof));
        // A roof block already at 8 73 6 offers its south face to an eye south of z 7.
        let side = BlockPos::new(8, 73, 6);
        assert!(face_visible(Vec3::new(8.5, 72.62, 7.5), side, roof));
        assert!(!face_visible(inside_eye, side, roof));
    }

    #[test]
    fn soft_natural_blocks_are_leaves_ground_and_flora_not_stone_or_builds() {
        for b in [
            "oak_leaves",
            "dirt",
            "grass_block",
            "sand",
            "gravel",
            "snow_block",
            "poppy",
            "red_tulip",
            "mud",
        ] {
            assert!(soft_natural(b), "{b}");
        }
        for b in [
            "stone",
            "iron_ore",
            "oak_planks",
            "cobblestone",
            "bedrock",
            "air",
        ] {
            assert!(!soft_natural(b), "{b}");
        }
    }

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
