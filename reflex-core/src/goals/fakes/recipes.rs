//! A small fixture recipe book in the vanilla 26.3 shape (tags expanded into item lists,
//! vanilla recipe ids), for planner tests and goal simulations.

use crate::crafting::{Ingredient, Recipe, RecipeBook, Shape};


pub fn tag(items: &[&str]) -> Ingredient {
    Ingredient::of(items)
}

pub fn shaped(id: &str, count: u32, rows: &[&str], key: &[(char, Ingredient)]) -> Recipe {
    let width = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let cells = rows
        .iter()
        .flat_map(|r| format!("{r:width$}").chars().collect::<Vec<_>>())
        .map(|c| key.iter().find(|(k, _)| *k == c).map(|(_, i)| i.clone()))
        .collect();
    let shape = Shape::Shaped {
        width: width as u8,
        height: rows.len() as u8,
        cells,
    };
    Recipe {
        id: id.into(),
        result: result_of(id),
        count,
        shape,
    }
}

fn result_of(id: &str) -> String {
    let id = id.strip_prefix("dye_").unwrap_or(id);
    id.split("_from_").next().unwrap_or(id).to_string()
}

pub fn shapeless(id: &str, count: u32, ings: Vec<Ingredient>) -> Recipe {
    Recipe {
        id: id.into(),
        result: result_of(id),
        count,
        shape: Shape::Shapeless(ings),
    }
}

pub fn smelt(id: &str, input: Ingredient) -> Recipe {
    Recipe {
        id: id.into(),
        result: result_of(id),
        count: 1,
        shape: Shape::Smelt { input, ticks: 200 },
    }
}

pub fn inv(items: &[(&str, u32)]) -> Vec<(String, u32)> {
    items.iter().map(|(n, c)| (n.to_string(), *c)).collect()
}

pub fn book() -> RecipeBook {
    let planks = tag(&["oak_planks", "birch_planks", "spruce_planks", "bamboo_planks"]);
    let oak_logs = tag(&["oak_log", "oak_wood"]);
    let birch_logs = tag(&["birch_log", "birch_wood"]);
    let logs = tag(&["oak_log", "oak_wood", "birch_log", "birch_wood", "spruce_log"]);
    let stone = tag(&["cobblestone", "blackstone", "cobbled_deepslate"]);
    let one = Ingredient::one;
    let mut b = RecipeBook::default();
    for (name, t) in [
        ("planks", &planks),
        ("oak_logs", &oak_logs),
        ("birch_logs", &birch_logs),
        ("logs", &logs),
        ("stone_tool_materials", &stone),
    ] {
        b.add_tag(name, t.0.clone());
    }
    let p = |c| (c, planks.clone());
    let s = |c| (c, one("stick"));
    for r in [
        shapeless("oak_planks", 4, vec![oak_logs.clone()]),
        shapeless("birch_planks", 4, vec![birch_logs.clone()]),
        shapeless("spruce_planks", 4, vec![one("spruce_log")]),
        shapeless("bamboo_planks", 2, vec![one("bamboo_block")]),
        shaped("oak_wood", 3, &["##", "##"], &[('#', one("oak_log"))]),
        shaped("birch_wood", 3, &["##", "##"], &[('#', one("birch_log"))]),
        shaped("stick", 4, &["#", "#"], &[p('#')]),
        shaped("stick_from_bamboo_item", 1, &["#", "#"], &[('#', one("bamboo"))]),
        shaped("crafting_table", 1, &["##", "##"], &[p('#')]),
        shaped("chest", 1, &["###", "# #", "###"], &[p('#')]),
        shaped("oak_door", 3, &["##", "##", "##"], &[('#', one("oak_planks"))]),
        shaped("furnace", 1, &["###", "# #", "###"], &[('#', stone.clone())]),
        shaped("stone_pickaxe", 1, &["XXX", " # ", " # "], &[('X', stone.clone()), s('#')]),
        shaped("iron_pickaxe", 1, &["XXX", " # ", " # "], &[('X', one("iron_ingot")), s('#')]),
        shaped("iron_sword", 1, &["X", "X", "#"], &[('X', one("iron_ingot")), s('#')]),
        shaped("wooden_axe", 1, &["XX", "X#", " #"], &[p('X'), s('#')]),
        smelt("iron_ingot_from_smelting_raw_iron", one("raw_iron")),
        smelt("iron_nugget_from_smelting", tag(&["iron_pickaxe", "iron_sword"])),
        smelt("charcoal", logs.clone()),
        shaped("iron_block", 1, &["###", "###", "###"], &[('#', one("iron_ingot"))]),
        shapeless("iron_ingot_from_iron_block", 9, vec![one("iron_block")]),
        shaped("iron_ingot_from_nuggets", 1, &["###", "###", "###"], &[('#', one("iron_nugget"))]),
        shapeless("iron_nugget", 9, vec![one("iron_ingot")]),
        shaped("torch", 4, &["X", "#"], &[('X', tag(&["coal", "charcoal"])), s('#')]),
        shaped("white_wool_from_string", 1, &["##", "##"], &[('#', one("string"))]),
        shapeless("dye_white_wool", 1, vec![one("white_dye"), tag(&["red_wool", "blue_wool"])]),
        shapeless("dye_red_wool", 1, vec![one("red_dye"), tag(&["white_wool", "blue_wool"])]),
        shaped("white_bed", 1, &["###", "XXX"], &[('#', one("white_wool")), p('X')]),
        shaped("bread", 1, &["###"], &[('#', one("wheat"))]),
    ] {
        b.add(r);
    }
    b.finish();
    b
}
