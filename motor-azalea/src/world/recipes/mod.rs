//! The vanilla recipe book, read once at bot start from the data pack inside Mojang's
//! server jar on this machine (Azalea ignores the server's recipe packets). Nothing derived
//! from the jar is written to disk. Without the jar the book is empty, and craft and smelt
//! goals fail with a clear reason.

mod parse;
mod stations;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use reflex_core::crafting::RecipeBook;
use serde_json::Value;

use parse::{bare, Tags};

/// Overrides the server jar the recipes are read from.
pub const JAR_ENV: &str = "ULTRON_SERVER_JAR";
/// Under `$HOME`: the jar of the local test server.
const DEFAULT_JAR: &str = "minecraft/vanilla-26.3/versions/26.3/server-26.3.jar";
const RECIPES: &str = "data/minecraft/recipe/";
const TAGS: &str = "data/minecraft/tags/item/";
/// Nested tag references followed before a cycle is assumed.
const TAG_DEPTH: u32 = 8;

static BOOK: OnceLock<Arc<RecipeBook>> = OnceLock::new();

/// The book, loaded (and logged) on first use.
pub fn book() -> Arc<RecipeBook> {
    BOOK.get_or_init(|| Arc::new(load_logged())).clone()
}

fn jar_path() -> PathBuf {
    match std::env::var_os(JAR_ENV) {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(DEFAULT_JAR),
    }
}

fn load_logged() -> RecipeBook {
    let path = jar_path();
    match load(&path) {
        Ok((book, tags, skipped)) => {
            let total: usize = skipped.values().sum();
            let mut kinds: Vec<_> = skipped.iter().collect();
            kinds.sort_by_key(|(k, n)| (std::cmp::Reverse(**n), (*k).clone()));
            let kinds: Vec<String> = kinds.iter().map(|(k, n)| format!("{k} {n}")).collect();
            eprintln!(
                "RECIPES loaded {} recipes and {tags} item tags from {} ({total} entries skipped: {})",
                book.len(),
                path.display(),
                kinds.join(", ")
            );
            book
        }
        Err(e) => {
            eprintln!(
                "RECIPES none: cannot read {} ({e}); craft and smelt goals will fail. Set {JAR_ENV}",
                path.display()
            );
            RecipeBook::default()
        }
    }
}

/// Recipe entries left out of the book, counted by kind (the recipe type, or the folder of
/// a nested entry such as `brewing`). `parse` says why each kind is left out.
pub type Skipped = BTreeMap<String, usize>;

/// The book, the number of item tags, and the recipe entries skipped.
pub fn load(path: &std::path::Path) -> Result<(RecipeBook, usize, Skipped), String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(BufReader::new(file)).map_err(|e| e.to_string())?;
    let mut raw_tags: HashMap<String, Vec<String>> = HashMap::new();
    let mut raw_recipes: Vec<(String, Value)> = Vec::new();
    let mut skipped = Skipped::new();
    let mut skip = |kind: &str| *skipped.entry(kind.to_string()).or_default() += 1;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = f.name().to_string();
        let (dir, is_tag) = match (name.strip_prefix(TAGS), name.strip_prefix(RECIPES)) {
            (Some(rest), _) => (rest, true),
            (None, Some(rest)) => (rest, false),
            _ => continue,
        };
        // Recipes sit at the top of the folder, except the brewing ones in `brewing/`.
        let flat = |id: &&str| !id.contains('/') || (!is_tag && id.starts_with("brewing/"));
        let Some(id) = dir.strip_suffix(".json").filter(flat) else {
            if !is_tag && !dir.is_empty() && !dir.ends_with('/') {
                skip(dir.split('/').next().unwrap_or("unreadable"));
            }
            continue;
        };
        let mut text = String::new();
        let json = f
            .read_to_string(&mut text)
            .ok()
            .and(serde_json::from_str::<Value>(&text).ok());
        match (json, is_tag) {
            (Some(v), true) => {
                let values = v["values"].as_array().cloned().unwrap_or_default();
                let names = values
                    .iter()
                    .filter_map(|e| e.as_str().or(e["id"].as_str()));
                raw_tags.insert(id.to_string(), names.map(String::from).collect());
            }
            (Some(v), false) => raw_recipes.push((id.to_string(), v)),
            (None, false) => skip("unreadable"),
            (None, true) => {}
        }
    }
    let tags: Tags = raw_tags
        .keys()
        .map(|k| (k.clone(), expand(k, &raw_tags, 0)))
        .collect();
    let mut book = RecipeBook::default();
    for (id, v) in &raw_recipes {
        match parse::recipe(id, v, &tags) {
            Some(r) => book.add(r),
            None => skip(bare(v["type"].as_str().unwrap_or("unreadable"))),
        }
    }
    // The one recipe the data pack does not hold: a bottle filled at water.
    book.add(reflex_core::crafting::bottle_fill());
    for (name, items) in &tags {
        book.add_tag(name, items.clone());
    }
    book.finish();
    Ok((book, tags.len(), skipped))
}

/// A tag's items with nested `#tag` references followed.
fn expand(tag: &str, raw: &HashMap<String, Vec<String>>, depth: u32) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for v in raw.get(tag).into_iter().flatten() {
        let items = match v.strip_prefix('#') {
            Some(inner) if depth < TAG_DEPTH => expand(bare(inner), raw, depth + 1),
            Some(_) => Vec::new(),
            None => vec![bare(v).to_string()],
        };
        for i in items {
            if !out.contains(&i) {
                out.push(i);
            }
        }
    }
    out
}
