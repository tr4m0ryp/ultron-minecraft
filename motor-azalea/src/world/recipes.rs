//! Built-in crafting recipes. Azalea ignores the server's recipe packets at the pinned
//! revision, so the common survival recipes are listed here (vanilla shapes, standard
//! values, not re-read from the 26.3 data). Keys are query kinds: "planks" accepts any wood.

use reflex_core::goals::Recipe;

pub struct Shaped {
    pub rows: Vec<&'static str>,
    pub key: Vec<(char, String)>,
    pub yields: u32,
}

impl Shaped {
    fn new(rows: &[&'static str], key: &[(char, &str)], yields: u32) -> Self {
        Self {
            rows: rows.to_vec(),
            key: key.iter().map(|(c, k)| (*c, k.to_string())).collect(),
            yields,
        }
    }

    pub fn width(&self) -> usize {
        self.rows.iter().map(|r| r.len()).max().unwrap_or(0)
    }

    pub fn height(&self) -> usize {
        self.rows.len()
    }

    pub fn ingredient(&self, c: char) -> Option<&str> {
        self.key
            .iter()
            .find(|(k, _)| *k == c)
            .map(|(_, v)| v.as_str())
    }

    pub fn recipe(&self) -> Recipe {
        let mut ingredients: Vec<(String, u32)> = Vec::new();
        for c in self
            .rows
            .iter()
            .flat_map(|r| r.chars())
            .filter(|c| *c != ' ')
        {
            let Some(name) = self.ingredient(c) else {
                continue;
            };
            match ingredients.iter_mut().find(|(n, _)| n == name) {
                Some((_, k)) => *k += 1,
                None => ingredients.push((name.to_string(), 1)),
            }
        }
        Recipe {
            ingredients,
            yields: self.yields,
            needs_table: self.width() > 2 || self.height() > 2,
        }
    }
}

const TOOL_SHAPES: [(&str, [&str; 3]); 5] = [
    ("pickaxe", ["MMM", " S ", " S "]),
    ("axe", ["MM", "MS", " S"]),
    ("shovel", ["M", "S", "S"]),
    ("sword", ["M", "M", "S"]),
    ("hoe", ["MM", " S", " S"]),
];

const MATERIALS: [(&str, &str); 6] = [
    ("wooden", "planks"),
    ("stone", "cobblestone"),
    ("copper", "copper_ingot"),
    ("iron", "iron_ingot"),
    ("golden", "gold_ingot"),
    ("diamond", "diamond"),
];

fn tool(item: &str) -> Option<Shaped> {
    let (material, class) = item.split_once('_')?;
    let (_, m) = MATERIALS.iter().find(|(p, _)| *p == material)?;
    let (_, rows) = TOOL_SHAPES.iter().find(|(c, _)| *c == class)?;
    Some(Shaped::new(rows, &[('M', m), ('S', "stick")], 1))
}

fn planks(item: &str) -> Option<Shaped> {
    let wood = item.strip_suffix("_planks")?;
    let log = match wood {
        "crimson" | "warped" => format!("{wood}_stem"),
        "bamboo" => "bamboo_block".to_string(),
        _ => format!("{wood}_log"),
    };
    Some(Shaped::new(&["L"], &[('L', &log)], 4))
}

pub fn lookup(item: &str) -> Option<Shaped> {
    if let Some(s) = planks(item).or_else(|| tool(item)) {
        return Some(s);
    }
    let p = ('P', "planks");
    Some(match item {
        "stick" => Shaped::new(&["P", "P"], &[p], 4),
        "crafting_table" => Shaped::new(&["PP", "PP"], &[p], 1),
        "torch" => Shaped::new(&["C", "S"], &[('C', "coal"), ('S', "stick")], 4),
        "chest" => Shaped::new(&["PPP", "P P", "PPP"], &[p], 1),
        "furnace" => Shaped::new(&["CCC", "C C", "CCC"], &[('C', "cobblestone")], 1),
        "bread" => Shaped::new(&["WWW"], &[('W', "wheat")], 1),
        "bucket" => Shaped::new(&["I I", " I "], &[('I', "iron_ingot")], 1),
        "shield" => Shaped::new(&["PIP", "PPP", " P "], &[p, ('I', "iron_ingot")], 1),
        "ladder" => Shaped::new(&["S S", "SSS", "S S"], &[('S', "stick")], 3),
        "white_bed" => Shaped::new(&["WWW", "PPP"], &[('W', "white_wool"), p], 1),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::lookup;

    #[test]
    fn grid_size_and_ingredients() {
        let stick = lookup("stick").unwrap().recipe();
        assert_eq!(stick.ingredients, vec![("planks".to_string(), 2)]);
        assert!(!stick.needs_table);
        let pick = lookup("iron_pickaxe").unwrap().recipe();
        assert!(pick.needs_table);
        assert_eq!(
            pick.ingredients,
            vec![("iron_ingot".to_string(), 3), ("stick".to_string(), 2)]
        );
        assert_eq!(lookup("crimson_planks").unwrap().key[0].1, "crimson_stem");
        assert!(lookup("beacon").is_none());
    }
}
