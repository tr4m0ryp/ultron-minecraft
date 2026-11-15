//! Brewing and smithing entries of the data pack. A brewing entry names its bottles by item
//! and potion contents; here each becomes one item name (`world::names::potion_name`), the
//! same name the inventory reads give a held bottle.

use reflex_core::crafting::{Ingredient, Recipe, Shape};
use serde_json::Value;

use super::parse::{bare, ingredient, Tags};
use crate::world::names::potion_name;

/// `{"item": "minecraft:potion", "potion_contents": {"potions": "minecraft:water"}}`.
fn bottle_in(v: &Value) -> Option<String> {
    let item = bare(v["item"].as_str()?);
    let potion = bare(v["potion_contents"]["potions"].as_str()?);
    Some(potion_name(item, potion))
}

/// `{"id": "minecraft:potion", "components": {"minecraft:potion_contents": {"potion": ..}}}`.
fn bottle_out(v: &Value) -> Option<String> {
    let item = bare(v["id"].as_str()?);
    let c = &v["components"];
    let contents = c.get("minecraft:potion_contents").or(c.get("potion_contents"))?;
    Some(potion_name(item, bare(contents["potion"].as_str()?)))
}

pub fn brewing(id: &str, v: &Value, tags: &Tags) -> Option<Recipe> {
    Some(Recipe {
        id: id.to_string(),
        result: bottle_out(&v["output"])?,
        count: 1,
        shape: Shape::Brew {
            input: Ingredient(vec![bottle_in(&v["input"])?]),
            reagent: ingredient(&v["reagent"], tags)?,
        },
    })
}

/// A smithing transform (the netherite upgrades). Trims are left out: their result is the
/// same item with a pattern on it, which item names do not carry.
pub fn smithing(id: &str, v: &Value, tags: &Tags) -> Option<Recipe> {
    Some(Recipe {
        id: id.to_string(),
        result: bare(v["result"]["id"].as_str()?).to_string(),
        count: 1,
        shape: Shape::Smith {
            template: ingredient(&v["template"], tags)?,
            base: ingredient(&v["base"], tags)?,
            addition: ingredient(&v["addition"], tags)?,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_brewing_entry_names_its_bottles_by_contents() {
        let v = json!({"type": "minecraft:brewing",
            "input": {"item": "minecraft:potion", "potion_contents": {"potions": "minecraft:water"}},
            "output": {"components": {"minecraft:potion_contents": {"potion": "minecraft:awkward"}},
                "id": "minecraft:potion"},
            "reagent": {"item": "minecraft:nether_wart"}});
        let r = brewing("brewing/potion_water_nether_wart", &v, &Tags::new()).expect("reads");
        assert_eq!(r.result, "awkward_potion");
        assert_eq!(r.station(), Some("brewing_stand"));
        let inputs: Vec<&str> = r.inputs().iter().map(|(i, _)| i.0[0].as_str()).collect();
        assert_eq!(inputs, ["water_potion", "nether_wart"]);
        let splash = json!({"type": "minecraft:brewing",
            "input": {"item": "minecraft:potion", "potion_contents": {"potions": "minecraft:healing"}},
            "output": {"components": {"minecraft:potion_contents": {"potion": "minecraft:healing"}},
                "id": "minecraft:splash_potion"},
            "reagent": {"item": "minecraft:gunpowder"}});
        let r = brewing("brewing/x", &splash, &Tags::new()).expect("reads");
        assert_eq!(r.result, "healing_splash_potion");
    }

    #[test]
    fn a_smithing_transform_reads_its_three_slots() {
        let mut tags = Tags::new();
        tags.insert("netherite_tool_materials".into(), vec!["netherite_ingot".into()]);
        let v = json!({"type": "minecraft:smithing_transform",
            "addition": "#minecraft:netherite_tool_materials", "base": "minecraft:diamond_pickaxe",
            "result": {"id": "minecraft:netherite_pickaxe"},
            "template": "minecraft:netherite_upgrade_smithing_template"});
        let r = smithing("netherite_pickaxe_smithing", &v, &tags).expect("reads");
        assert_eq!((r.result.as_str(), r.station()), ("netherite_pickaxe", Some("smithing_table")));
        assert_eq!(r.inputs().len(), 3);
    }
}
