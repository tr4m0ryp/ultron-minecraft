//! Fixture recipes for the brewing stand and the smithing table, in the 26.3 shape: a potion
//! is named by its contents ("water_potion", "healing_potion", "healing_splash_potion").

use crate::crafting::{Ingredient, Recipe, RecipeBook, Shape};

use super::recipes::{shaped, shapeless, tag};

fn brew(input: &str, reagent: &str, result: &str) -> Recipe {
    Recipe {
        id: format!("brewing/{result}_from_{input}"),
        result: result.into(),
        count: 1,
        shape: Shape::Brew {
            input: Ingredient::one(input),
            reagent: Ingredient::one(reagent),
        },
    }
}

fn smith(base: &str, result: &str) -> Recipe {
    Recipe {
        id: format!("{result}_smithing"),
        result: result.into(),
        count: 1,
        shape: Shape::Smith {
            template: Ingredient::one("netherite_upgrade_smithing_template"),
            base: Ingredient::one(base),
            addition: Ingredient::one("netherite_ingot"),
        },
    }
}

pub fn add(b: &mut RecipeBook) {
    let one = Ingredient::one;
    let stone = tag(&["cobblestone", "blackstone", "cobbled_deepslate"]);
    let planks = tag(&["oak_planks", "birch_planks", "spruce_planks"]);
    for r in [
        brew("water_potion", "nether_wart", "awkward_potion"),
        brew("awkward_potion", "glistering_melon_slice", "healing_potion"),
        brew("healing_potion", "glowstone_dust", "strong_healing_potion"),
        brew("healing_potion", "gunpowder", "healing_splash_potion"),
        smith("diamond_pickaxe", "netherite_pickaxe"),
        smith("diamond_sword", "netherite_sword"),
        shaped(
            "brewing_stand",
            1,
            &[" B ", "###"],
            &[('B', one("blaze_rod")), ('#', stone)],
        ),
        shapeless("blaze_powder", 2, vec![one("blaze_rod")]),
        shaped(
            "smithing_table",
            1,
            &["II", "##", "##"],
            &[('I', one("iron_ingot")), ('#', planks)],
        ),
        shaped(
            "glistering_melon_slice",
            1,
            &["NNN", "NMN", "NNN"],
            &[('N', one("gold_nugget")), ('M', one("melon_slice"))],
        ),
        shaped(
            "diamond_pickaxe",
            1,
            &["XXX", " # ", " # "],
            &[('X', one("diamond")), ('#', one("stick"))],
        ),
    ] {
        b.add(r);
    }
}
