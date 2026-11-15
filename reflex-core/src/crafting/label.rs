//! Short names for sets of items in text meant for thought: "oak_log", "coal/charcoal",
//! "any plank" (a tag that covers the set), "any wool" (one family).

use super::book::RecipeBook;

fn singular(word: &str) -> &str {
    match word.strip_suffix('s') {
        Some(w) if !w.ends_with('s') && !w.is_empty() => w,
        _ => word,
    }
}

/// The name of a set of acceptable items.
pub fn name_set(book: &RecipeBook, items: &[String]) -> String {
    match items {
        [] => "nothing".into(),
        [one] => one.clone(),
        few if few.len() <= 3 => few.join("/"),
        many => {
            let covering = book
                .tags()
                .iter()
                .filter(|(_, t)| !t.is_empty() && t.iter().all(|i| many.contains(i)))
                .filter(|(_, t)| t.len() * 4 >= many.len() * 3)
                .max_by_key(|(_, t)| t.len());
            if let Some((tag, _)) = covering {
                return format!("any {}", singular(tag));
            }
            let fam = |s: &str| s.rsplit('_').next().unwrap_or(s).to_string();
            let first = fam(&many[0]);
            if many.iter().all(|i| fam(i) == first) {
                return format!("any {}", singular(&first));
            }
            format!("one of {} kinds such as {}", many.len(), many[0])
        }
    }
}
