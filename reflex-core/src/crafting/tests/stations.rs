//! The planner over brewing and smithing recipes: batches of three bottles on one reagent,
//! blaze powder for a new stand, stations crafted or held, and the shopping list.

use super::{book, inv};
use crate::crafting::{plan, CraftPlan, PlanStep, Stations};

const NONE: Stations = Stations {
    table: false,
    furnace: false,
    stand: false,
    smithing: false,
    stand_fuelled: false,
};
const STAND: Stations = Stations {
    stand: true,
    stand_fuelled: true,
    ..NONE
};

fn steps(p: &CraftPlan) -> Vec<String> {
    p.steps.iter().map(PlanStep::describe).collect()
}

#[test]
fn three_bottles_brew_on_one_reagent_through_the_awkward_potion() {
    let kit = inv(&[
        ("water_potion", 3),
        ("nether_wart", 1),
        ("glistering_melon_slice", 1),
    ]);
    let p = plan(&book(), &kit, "healing_potion", 3, STAND, false).expect("plans");
    assert_eq!(
        steps(&p),
        ["brew 3 awkward_potion", "brew 3 healing_potion"]
    );
    let four = plan(&book(), &kit, "awkward_potion", 4, STAND, false).unwrap_err();
    assert_eq!(
        four,
        "cannot craft awkward_potion: need 4 water_potion, have 3; need 2 nether_wart, have 1. \
         Get 1 water_potion, 1 nether_wart."
    );
}

#[test]
fn a_stand_that_is_new_or_known_empty_needs_blaze_powder() {
    let kit = inv(&[("water_potion", 1), ("nether_wart", 1)]);
    let empty = Stations {
        stand_fuelled: false,
        ..STAND
    };
    let e = plan(&book(), &kit, "awkward_potion", 1, empty, false).unwrap_err();
    assert_eq!(
        e,
        "cannot craft awkward_potion: need 1 blaze_powder, have 0: craft 2 from 1 blaze_rod; \
         need 1 blaze_rod, have 0. Get 1 blaze_rod."
    );
    let mut with = kit.clone();
    with.extend(inv(&[("blaze_powder", 1), ("brewing_stand", 1)]));
    let p = plan(&book(), &with, "awkward_potion", 1, NONE, false).expect("held stand and powder");
    assert_eq!(steps(&p), ["brew 1 awkward_potion"]);
}

#[test]
fn a_missing_stand_is_crafted_first_with_its_table() {
    let kit = inv(&[
        ("water_potion", 1),
        ("nether_wart", 1),
        ("blaze_rod", 2),
        ("cobblestone", 3),
        ("oak_log", 1),
    ]);
    let p = plan(&book(), &kit, "awkward_potion", 1, NONE, false).expect("plans");
    assert_eq!(
        steps(&p),
        [
            "craft 4 oak_planks",
            "craft 1 crafting_table",
            "craft 1 brewing_stand",
            "craft 2 blaze_powder",
            "brew 1 awkward_potion"
        ]
    );
}

#[test]
fn an_impossible_potion_gives_the_whole_shopping_list() {
    let kit = inv(&[("water_potion", 1)]);
    let e = plan(&book(), &kit, "healing_potion", 1, STAND, false).unwrap_err();
    assert_eq!(
        e,
        "cannot craft healing_potion: need 1 crafting_table, have 0: craft 1 from 4 any plank; \
         need 4 any plank, have 0: craft 4 from 1 any log; need 1 any log, have 0; \
         need 1 awkward_potion, have 0: brew 1 from 1 water_potion \
         and 1 nether_wart; need 1 nether_wart, have 0; need 1 glistering_melon_slice, have 0: \
         craft 1 from 8 gold_nugget and 1 melon_slice; need 8 gold_nugget, have 0; \
         need 1 melon_slice, have 0. Get 1 any log, 1 nether_wart, 8 gold_nugget, 1 melon_slice."
    );
}

#[test]
fn a_diamond_pickaxe_is_upgraded_at_a_smithing_table() {
    let kit = inv(&[
        ("diamond_pickaxe", 1),
        ("netherite_upgrade_smithing_template", 1),
        ("netherite_ingot", 1),
    ]);
    let near = Stations {
        smithing: true,
        ..NONE
    };
    let p = plan(&book(), &kit, "netherite_pickaxe", 1, near, false).expect("plans");
    assert_eq!(steps(&p), ["smith 1 netherite_pickaxe"]);
    let mut raw = kit.clone();
    raw.extend(inv(&[("iron_ingot", 2), ("oak_log", 2)]));
    let p = plan(&book(), &raw, "netherite_pickaxe", 1, NONE, false).expect("makes its table");
    assert_eq!(
        steps(&p),
        [
            "craft 4 oak_planks",
            "craft 1 crafting_table",
            "craft 4 oak_planks",
            "craft 1 smithing_table",
            "smith 1 netherite_pickaxe"
        ]
    );
    let bare = inv(&[("diamond_pickaxe", 1)]);
    assert_eq!(
        plan(&book(), &bare, "netherite_pickaxe", 1, near, false).unwrap_err(),
        "cannot craft netherite_pickaxe: need 1 netherite_upgrade_smithing_template, have 0; \
         need 1 netherite_ingot, have 0. Get 1 netherite_upgrade_smithing_template, \
         1 netherite_ingot."
    );
}
