//! Fixture recipes for deep chains, in the vanilla 26.3 shape: gold with its nuggets, its
//! storage blocks and the furnace list that melts golden gear down (horse armour in it is
//! only ever found), netherite, and the things an ore drops with the recipes that also give
//! them (unpacking a storage block, smelting the ore block).

use crate::crafting::{Ingredient, RecipeBook};

use super::recipes::{shaped, shapeless, smelt, tag};

pub fn add(b: &mut RecipeBook) {
    let one = Ingredient::one;
    let nine = |id: &str, item: &str| shaped(id, 1, &["###", "###", "###"], &[('#', one(item))]);
    let tool = |id: &str, head: &str| {
        shaped(
            id,
            1,
            &["XXX", " # ", " # "],
            &[('X', one(head)), ('#', one("stick"))],
        )
    };
    for r in [
        shapeless("gold_nugget", 9, vec![one("gold_ingot")]),
        nine("gold_ingot_from_nuggets", "gold_nugget"),
        nine("gold_block", "gold_ingot"),
        shapeless("gold_ingot_from_gold_block", 9, vec![one("gold_block")]),
        smelt("gold_ingot_from_smelting_gold_ore", one("gold_ore")),
        smelt("gold_ingot_from_smelting_raw_gold", one("raw_gold")),
        nine("raw_gold_block", "raw_gold"),
        shapeless("raw_gold", 9, vec![one("raw_gold_block")]),
        tool("golden_pickaxe", "gold_ingot"),
        shaped(
            "golden_sword",
            1,
            &["X", "X", "#"],
            &[('X', one("gold_ingot")), ('#', one("stick"))],
        ),
        smelt(
            "gold_nugget_from_smelting",
            tag(&["golden_pickaxe", "golden_sword", "golden_horse_armor"]),
        ),
        shaped(
            "netherite_upgrade_smithing_template",
            2,
            &["#S#", "#C#", "###"],
            &[
                ('#', one("diamond")),
                ('S', one("netherite_upgrade_smithing_template")),
                ('C', one("netherrack")),
            ],
        ),
        shapeless(
            "netherite_ingot",
            1,
            [vec![one("netherite_scrap"); 4], vec![one("gold_ingot"); 4]].concat(),
        ),
        smelt("netherite_scrap", one("ancient_debris")),
        nine("coal_block", "coal"),
        shapeless("coal", 9, vec![one("coal_block")]),
        smelt("coal_from_smelting_coal_ore", one("coal_ore")),
        nine("diamond_block", "diamond"),
        shapeless("diamond", 9, vec![one("diamond_block")]),
        smelt("diamond_from_smelting_diamond_ore", one("diamond_ore")),
        smelt("iron_ingot_from_smelting_iron_ore", one("iron_ore")),
    ] {
        b.add(r);
    }
}
