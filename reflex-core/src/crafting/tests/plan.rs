use super::{book, inv};
use crate::crafting::{plan, CraftPlan, PlanStep, Stations};

const NONE: Stations = Stations {
    table: false,
    furnace: false,
};
const BOTH: Stations = Stations {
    table: true,
    furnace: true,
};

fn steps(p: &CraftPlan) -> Vec<String> {
    p.steps.iter().map(PlanStep::describe).collect()
}

fn ok(items: &[(&str, u32)], item: &str, n: u32, near: Stations) -> CraftPlan {
    plan(&book(), &inv(items), item, n, near, false).unwrap_or_else(|e| panic!("{item}: {e}"))
}

fn err(items: &[(&str, u32)], item: &str, n: u32, near: Stations) -> String {
    plan(&book(), &inv(items), item, n, near, false).expect_err(item)
}

#[test]
fn stone_pickaxe_from_logs_crafts_its_own_table_first() {
    let p = ok(&[("oak_log", 2), ("cobblestone", 3)], "stone_pickaxe", 1, NONE);
    assert_eq!(
        steps(&p),
        [
            "craft 4 oak_planks",
            "craft 1 crafting_table",
            "craft 4 oak_planks",
            "craft 4 stick",
            "craft 1 stone_pickaxe"
        ]
    );
    assert!(p.needs_table && !p.needs_furnace);
    let near = ok(&[("oak_log", 2), ("cobblestone", 3)], "stone_pickaxe", 1, BOTH);
    assert_eq!(near.steps.len(), 3, "a table in reach is not crafted");
}

#[test]
fn alternatives_follow_the_inventory() {
    let birch = ok(&[("birch_log", 1)], "stick", 4, NONE);
    assert_eq!(steps(&birch), ["craft 4 birch_planks", "craft 4 stick"]);
    let bamboo = ok(&[("bamboo", 4)], "stick", 2, NONE);
    assert_eq!(steps(&bamboo), ["craft 2 stick"]);
    assert_eq!(bamboo.steps[0].recipe().id, "stick_from_bamboo_item");
    let held = ok(&[("spruce_planks", 2), ("oak_log", 5)], "stick", 4, NONE);
    assert_eq!(steps(&held), ["craft 4 stick"], "held planks before logs");
    let wood = ok(&[("oak_wood", 1)], "oak_planks", 4, NONE);
    assert_eq!(steps(&wood), ["craft 4 oak_planks"], "oak_wood is in #oak_logs");
}

#[test]
fn tag_ingredients_accept_any_member() {
    let p = ok(&[("blackstone", 3), ("stick", 2)], "stone_pickaxe", 1, BOTH);
    assert_eq!(steps(&p), ["craft 1 stone_pickaxe"]);
    let mixed = ok(
        &[("cobblestone", 1), ("cobbled_deepslate", 2), ("stick", 2)],
        "stone_pickaxe",
        1,
        BOTH,
    );
    assert_eq!(mixed.steps.len(), 1);
}

#[test]
fn counts_follow_yields_and_reuse_surplus() {
    let door = ok(&[("oak_log", 3)], "oak_door", 3, BOTH);
    assert_eq!(steps(&door), ["craft 8 oak_planks", "craft 3 oak_door"]);
    let doors = ok(&[("oak_log", 3)], "oak_door", 4, BOTH);
    assert_eq!(steps(&doors), ["craft 12 oak_planks", "craft 6 oak_door"]);
    let torch = ok(&[("coal", 2), ("oak_log", 1)], "torch", 8, NONE);
    assert_eq!(
        steps(&torch),
        ["craft 4 oak_planks", "craft 4 stick", "craft 8 torch"]
    );
    // The chest takes 8 planks, the table 4: three logs, one plank craft each.
    let chest = ok(&[("oak_log", 3)], "chest", 1, NONE);
    let planks = chest.steps.iter().filter(|s| s.recipe().result == "oak_planks");
    assert_eq!(planks.map(PlanStep::makes).sum::<u32>(), 12);
    let short = err(&[("oak_log", 2)], "chest", 1, NONE);
    assert!(short.contains("need 2 any log, have 1"), "{short}");
}

#[test]
fn storage_blocks_and_nuggets_run_only_from_hand() {
    let unpack = ok(&[("iron_block", 1)], "iron_ingot", 3, NONE);
    assert_eq!(steps(&unpack), ["craft 9 iron_ingot"]);
    let pack = ok(&[("iron_ingot", 9)], "iron_block", 1, BOTH);
    assert_eq!(steps(&pack), ["craft 1 iron_block"]);
    // No ingots in hand: the block is not built from smelted ore, and nothing loops.
    let e = err(&[("raw_iron", 9), ("coal", 2)], "iron_block", 1, BOTH);
    assert!(e.contains("need 9 iron_ingot, have 0"), "{e}");
    let nug = err(&[], "iron_nugget", 1, BOTH);
    assert!(!nug.contains("iron_sword"), "no recycling detour: {nug}");
}

#[test]
fn recolouring_is_not_chained() {
    let e = err(&[("oak_planks", 3)], "white_bed", 1, BOTH);
    assert_eq!(
        e,
        "cannot craft white_bed: need 3 white_wool, have 0: craft 3 from 12 string; \
         need 12 string, have 0. Get 12 string."
    );
    let dyed = ok(&[("white_dye", 1), ("red_wool", 1)], "white_wool", 1, NONE);
    assert_eq!(steps(&dyed), ["craft 1 white_wool"], "dyeing from hand is fine");
}

#[test]
fn the_shopping_list_names_every_missing_link() {
    let e = err(&[("cobblestone", 1)], "stone_pickaxe", 1, NONE);
    assert_eq!(
        e,
        "cannot craft stone_pickaxe: need 1 crafting_table, have 0: craft 1 from 4 any plank; \
         need 4 any plank, have 0: craft 4 from 1 any log; need 1 any log, have 0; \
         need 3 cobblestone/blackstone/cobbled_deepslate, have 1; \
         need 2 stick, have 0: craft 4 from 2 any plank; \
         need 2 any plank, have 0: craft 4 from 1 any log; need 1 any log, have 0. \
         Get 2 any log, 2 cobblestone/blackstone/cobbled_deepslate."
    );
    assert_eq!(err(&[], "beacon", 1, NONE), "no recipe makes beacon");
}

#[test]
fn smelting_adds_a_furnace_a_table_and_fuel() {
    let kit = [
        ("raw_iron", 3),
        ("coal", 1),
        ("oak_log", 3),
        ("cobblestone", 8),
    ];
    let p = ok(&kit, "iron_pickaxe", 1, NONE);
    assert_eq!(
        steps(&p),
        [
            "craft 4 oak_planks",
            "craft 1 crafting_table",
            "craft 1 furnace",
            "smelt 3 iron_ingot",
            "craft 4 oak_planks",
            "craft 4 stick",
            "craft 1 iron_pickaxe"
        ]
    );
    assert!(p.needs_table && p.needs_furnace);
    let smelt = p.steps.iter().find(|s| matches!(s, PlanStep::Smelt { .. }));
    let Some(PlanStep::Smelt { fuel, count, .. }) = smelt else {
        panic!("no smelt step");
    };
    assert_eq!((*count, fuel.clone()), (3, vec![("coal".to_string(), 1)]));
    let e = err(&[("raw_iron", 3), ("stick", 2)], "iron_pickaxe", 1, BOTH);
    assert!(
        e.ends_with("need fuel for 3 more smelts: 1 coal or 2 logs or planks. Get 1 coal."),
        "{e}"
    );
    let logs = ok(&[("raw_iron", 3), ("oak_log", 3), ("stick", 2)], "iron_pickaxe", 1, BOTH);
    let Some(PlanStep::Smelt { fuel, .. }) = logs.steps.first() else {
        panic!("smelt first");
    };
    assert_eq!(fuel, &vec![("oak_log".to_string(), 2)]);
    let no_cobble = err(&[("raw_iron", 3), ("coal", 1), ("stick", 2)], "iron_pickaxe", 1, NONE);
    assert!(no_cobble.contains("need 1 furnace, have 0"), "{no_cobble}");
}

#[test]
fn smelt_only_restricts_the_last_step() {
    let b = book();
    let near = BOTH;
    let p = plan(&b, &inv(&[("iron_block", 1), ("raw_iron", 2), ("coal", 1)]), "iron_ingot", 2, near, true);
    let p = p.expect("smelts raw iron");
    assert_eq!(steps(&p), ["smelt 2 iron_ingot"]);
    let e = plan(&b, &inv(&[("iron_block", 1)]), "iron_ingot", 1, near, true).unwrap_err();
    assert!(e.starts_with("cannot smelt iron_ingot: need 1 raw_iron, have 0"), "{e}");
    let none = plan(&b, &[], "stick", 1, near, true).unwrap_err();
    assert_eq!(none, "no smelting recipe makes stick");
}
