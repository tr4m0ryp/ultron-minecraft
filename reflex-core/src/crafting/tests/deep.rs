//! Shopping lists for deep chains: the route named is the cheapest natural one (raw
//! materials, what an ore drops), never melting tools down or copying the thing itself.

use super::{book, inv};
use crate::crafting::{plan, Stations};

const ALL: Stations = Stations {
    table: true,
    furnace: true,
    stand: true,
    smithing: true,
    stand_fuelled: true,
};

fn list(items: &[(&str, u32)], item: &str) -> String {
    plan(&book(), &inv(items), item, 1, ALL, false).expect_err(item)
}

#[test]
fn gold_nuggets_come_from_one_ingot_smelted_from_raw_gold() {
    assert_eq!(
        list(&[], "glistering_melon_slice"),
        "cannot craft glistering_melon_slice: need 8 gold_nugget, have 0: craft 9 from 1 \
         gold_ingot; need 1 gold_ingot, have 0: smelt 1 from 1 raw_gold; need 1 raw_gold, \
         have 0; need 1 melon_slice, have 0; need fuel for 1 more smelt: 1 coal or 1 log or \
         plank. Get 1 raw_gold, 1 melon_slice, 1 coal."
    );
    // With the ingot held the nuggets are simply crafted.
    let p = plan(
        &book(),
        &inv(&[("gold_ingot", 1), ("melon_slice", 1)]),
        "glistering_melon_slice",
        1,
        ALL,
        false,
    )
    .expect("plans");
    assert_eq!(p.steps.len(), 2);
}

#[test]
fn a_potion_chain_never_asks_for_golden_tools() {
    let e = list(&[("water_potion", 1)], "healing_potion");
    assert!(
        !e.contains("golden_") && !e.contains("kinds such as"),
        "{e}"
    );
    assert!(
        e.ends_with("Get 1 nether_wart, 1 raw_gold, 1 melon_slice, 1 coal."),
        "{e}"
    );
}

#[test]
fn a_template_is_fetched_not_copied_from_itself() {
    let e = list(&[("diamond_pickaxe", 1)], "netherite_pickaxe");
    assert!(
        e.starts_with(
            "cannot craft netherite_pickaxe: need 1 netherite_upgrade_smithing_template, \
             have 0; need 1 netherite_ingot, have 0: craft 1 from 4 netherite_scrap and 4 \
             gold_ingot"
        ),
        "{e}"
    );
    assert!(e.contains("smelt 4 from 4 raw_gold"), "{e}");
    assert!(
        e.ends_with(
            "Get 1 netherite_upgrade_smithing_template, 4 ancient_debris, 4 raw_gold, 1 coal."
        ),
        "{e}"
    );
}

#[test]
fn what_an_ore_drops_is_fetched_not_smelted_from_the_ore_block() {
    let torch = list(&[], "torch");
    assert!(torch.contains("need 1 coal/charcoal, have 0;"), "{torch}");
    assert!(
        !torch.contains("coal_ore") && !torch.contains("coal_block"),
        "{torch}"
    );
    assert_eq!(
        list(&[("stick", 2)], "diamond_pickaxe"),
        "cannot craft diamond_pickaxe: need 3 diamond, have 0. Get 3 diamond."
    );
    // Held as a storage block, it is unpacked as before.
    let p = plan(
        &book(),
        &inv(&[("stick", 2), ("diamond_block", 1)]),
        "diamond_pickaxe",
        1,
        ALL,
        false,
    )
    .expect("plans");
    assert_eq!(p.steps.len(), 2);
}

#[test]
fn iron_comes_from_raw_iron_not_from_ore_blocks_or_nuggets() {
    let e = list(&[("stick", 2)], "iron_pickaxe");
    assert_eq!(
        e,
        "cannot craft iron_pickaxe: need 3 iron_ingot, have 0: smelt 3 from 3 raw_iron; \
         need 3 raw_iron, have 0; need fuel for 3 more smelts: 1 coal or 2 logs or planks. \
         Get 3 raw_iron, 1 coal."
    );
    let block = list(&[], "iron_block");
    assert!(
        block.contains("need 9 iron_ingot, have 0: smelt 9 from 9 raw_iron"),
        "{block}"
    );
}
