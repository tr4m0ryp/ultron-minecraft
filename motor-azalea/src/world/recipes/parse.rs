//! One data-pack recipe file into the planner's `Recipe`: shaped and shapeless crafting,
//! transmuting (dyeing a shulker box or a bundle, two items in the grid), smelting, brewing
//! and smithing transforms (`stations`), with tag and list ingredients expanded into
//! concrete items.
//!
//! Skipped, by kind (counts for 26.3, 444 in all):
//! - stonecutting (351), blasting (25), smoking (9), campfire_cooking (9): another station
//!   for a result the grid or the furnace already makes. Every cooked result has a smelting
//!   recipe; every cut one a crafting recipe, except cobblestone and cobbled deepslate,
//!   which are mined.
//! - smithing_trim (18): the result is the same armour with a pattern on it, which item
//!   names do not carry.
//! - crafting_dye (6), banner duplicate (16), map cloning (1, a transmute with no named
//!   result), shield decoration, map extending, book cloning, repair, firework star, star
//!   fade and rocket, decorated pot, tipped arrow (9): the result depends on colour,
//!   pattern, damage or contents, which item names do not carry. The plain rocket and the
//!   plain pot are ordinary recipes and are read.

use std::collections::HashMap;

use reflex_core::crafting::{Ingredient, Recipe, Shape};
use serde_json::Value;

/// Resolved item tags: bare tag name to bare item names.
pub type Tags = HashMap<String, Vec<String>>;

pub fn bare(s: &str) -> &str {
    s.strip_prefix("minecraft:").unwrap_or(s)
}

/// "minecraft:stick", "#minecraft:planks", a list of either, or the older object forms.
pub fn ingredient(v: &Value, tags: &Tags) -> Option<Ingredient> {
    let mut items: Vec<String> = Vec::new();
    let mut add = |name: &str| {
        if !items.iter().any(|i| i == name) {
            items.push(name.to_string());
        }
    };
    match v {
        Value::String(s) => match s.strip_prefix('#') {
            Some(tag) => tags.get(bare(tag))?.iter().for_each(|i| add(i)),
            None => add(bare(s)),
        },
        Value::Array(list) => {
            for e in list {
                ingredient(e, tags)?.0.iter().for_each(|i| add(i));
            }
        }
        Value::Object(o) => {
            if let Some(Value::String(i)) = o.get("item") {
                add(bare(i));
            } else if let Some(Value::String(t)) = o.get("tag") {
                tags.get(bare(t))?.iter().for_each(|i| add(i));
            }
        }
        _ => {}
    }
    (!items.is_empty()).then_some(Ingredient(items))
}

fn result(v: &Value) -> Option<(String, u32)> {
    match v {
        Value::String(s) => Some((bare(s).to_string(), 1)),
        Value::Object(o) => {
            let id = o.get("id").or(o.get("item"))?.as_str()?;
            let count = o.get("count").and_then(Value::as_u64).unwrap_or(1);
            Some((bare(id).to_string(), count as u32))
        }
        _ => None,
    }
}

fn shaped(v: &Value, tags: &Tags) -> Option<Shape> {
    let rows: Vec<&str> = v["pattern"]
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let key = v["key"].as_object()?;
    let width = rows.iter().map(|r| r.chars().count()).max()?;
    let mut cells = Vec::with_capacity(width * rows.len());
    for row in &rows {
        let mut chars: Vec<char> = row.chars().collect();
        chars.resize(width, ' ');
        for c in chars {
            cells.push(match c {
                ' ' => None,
                c => Some(ingredient(key.get(&c.to_string())?, tags)?),
            });
        }
    }
    Some(Shape::Shaped {
        width: width as u8,
        height: rows.len() as u8,
        cells,
    })
}

/// `None` for types the planner does not use, or an entry it cannot read.
pub fn recipe(id: &str, v: &Value, tags: &Tags) -> Option<Recipe> {
    let kind = bare(v["type"].as_str()?);
    let shape = match kind {
        "crafting_shaped" => shaped(v, tags)?,
        "crafting_shapeless" => {
            let list = v["ingredients"].as_array()?;
            let ings: Option<Vec<Ingredient>> = list.iter().map(|e| ingredient(e, tags)).collect();
            Shape::Shapeless(ings?)
        }
        "crafting_transmute" => Shape::Shapeless(vec![
            ingredient(&v["input"], tags)?,
            ingredient(&v["material"], tags)?,
        ]),
        "smelting" => Shape::Smelt {
            input: ingredient(&v["ingredient"], tags)?,
            ticks: v["cookingtime"].as_u64().unwrap_or(200) as u32,
        },
        "brewing" => return super::stations::brewing(id, v, tags),
        "smithing_transform" => return super::stations::smithing(id, v, tags),
        _ => return None,
    };
    let (result, count) = result(&v["result"])?;
    Some(Recipe {
        id: id.to_string(),
        result,
        count,
        shape,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn shaped_shapeless_smelting_and_tags() {
        let mut tags = Tags::new();
        tags.insert(
            "planks".into(),
            vec!["oak_planks".into(), "birch_planks".into()],
        );
        let stick = json!({"type": "minecraft:crafting_shaped", "key": {"#": "#minecraft:planks"},
            "pattern": ["#", "#"], "result": {"count": 4, "id": "minecraft:stick"}});
        let r = recipe("stick", &stick, &tags).expect("shaped");
        assert_eq!(
            (r.result.as_str(), r.count, r.needs_table()),
            ("stick", 4, false)
        );
        assert_eq!(r.inputs()[0].0 .0, ["oak_planks", "birch_planks"]);
        let pick = json!({"type": "minecraft:crafting_shaped", "pattern": ["XXX", " # ", " # "],
            "key": {"#": "minecraft:stick", "X": ["minecraft:coal", "minecraft:charcoal"]},
            "result": {"id": "minecraft:x"}});
        let r = recipe("x", &pick, &tags).expect("list key");
        assert!(r.needs_table());
        assert_eq!(r.cells(3).len(), 5);
        let dye = json!({"type": "minecraft:crafting_shapeless", "result": {"id": "minecraft:w"},
            "ingredients": ["minecraft:white_dye", ["minecraft:red_wool", "minecraft:blue_wool"]]});
        assert_eq!(
            recipe("w", &dye, &tags).expect("shapeless").inputs().len(),
            2
        );
        let iron = json!({"type": "minecraft:smelting", "cookingtime": 200,
            "ingredient": "minecraft:raw_iron", "result": {"id": "minecraft:iron_ingot"}});
        assert!(recipe("i", &iron, &tags).expect("smelt").is_smelt());
        let blast = json!({"type": "minecraft:blasting", "ingredient": "minecraft:raw_iron",
            "result": {"id": "minecraft:iron_ingot"}});
        assert!(recipe("b", &blast, &tags).is_none());
        tags.insert(
            "shulker_boxes".into(),
            vec!["shulker_box".into(), "black_shulker_box".into()],
        );
        let dyed = json!({"type": "minecraft:crafting_transmute",
            "input": "#minecraft:shulker_boxes", "material": "minecraft:black_dye",
            "result": {"id": "minecraft:black_shulker_box"}});
        let r = recipe("black_shulker_box", &dyed, &tags).expect("transmute");
        assert_eq!((r.inputs().len(), r.needs_table()), (2, false));
        let unknown = json!({"type": "minecraft:crafting_shaped", "pattern": ["#"],
            "key": {"#": "#minecraft:nope"}, "result": {"id": "minecraft:x"}});
        assert!(recipe("u", &unknown, &tags).is_none());
    }
}
