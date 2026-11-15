//! Which animals yield an item, and which one in view to take it from.

use crate::math::Vec3;
use crate::snapshot::{Entity, EntityKind};

/// Animals further than this are not hunted (the sensing radius).
pub const SOURCE_RADIUS: f64 = 32.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// Shears on a sheep: wool without a kill.
    Shear,
    Kill,
}

/// The item (a registry name, or "wool" for every colour), the animals that drop it, and
/// whether shears take it from a living one.
struct Yield {
    item: &'static str,
    from: &'static [EntityKind],
    shear: bool,
}

const YIELDS: &[Yield] = {
    use EntityKind::*;
    &[
        Yield {
            item: "wool",
            from: &[Sheep],
            shear: true,
        },
        Yield {
            item: "mutton",
            from: &[Sheep],
            shear: false,
        },
        Yield {
            item: "beef",
            from: &[Cow],
            shear: false,
        },
        Yield {
            item: "leather",
            from: &[Cow],
            shear: false,
        },
        Yield {
            item: "porkchop",
            from: &[Pig],
            shear: false,
        },
        Yield {
            item: "chicken",
            from: &[Chicken],
            shear: false,
        },
        Yield {
            item: "feather",
            from: &[Chicken],
            shear: false,
        },
        Yield {
            item: "rabbit",
            from: &[Rabbit],
            shear: false,
        },
        Yield {
            item: "rabbit_hide",
            from: &[Rabbit],
            shear: false,
        },
        Yield {
            item: "rabbit_foot",
            from: &[Rabbit],
            shear: false,
        },
    ]
};

/// Exact names only: "cooked_beef" is smelted, not hunted.
fn row(item: &str) -> Option<&'static Yield> {
    YIELDS
        .iter()
        .find(|y| item == y.item || (y.item == "wool" && item.ends_with("_wool")))
}

/// The animals that drop `item`, `None` when no animal does.
pub fn sources(item: &str) -> Option<&'static [EntityKind]> {
    row(item).map(|y| y.from)
}

/// The animal to take `item` from, or why there is none.
#[derive(Clone, Debug, PartialEq)]
pub enum Choice {
    Take {
        id: u32,
        method: Method,
    },
    /// Animals drop the item but none in view qualifies; the reason names them.
    NoneInRange(String),
    /// No animal drops this item.
    NotFromAnimals,
}

/// Ids the goal has learned to treat differently: shearing did nothing (`no_shear`, they
/// may still be killed) or the hunt gave up on them (`avoid`).
#[derive(Clone, Debug, Default)]
pub struct Memory {
    pub no_shear: Vec<u32>,
    pub avoid: Vec<u32>,
}

/// The nearest adult animal that yields `item`. Babies drop nothing; a sheared sheep has
/// no wool. With shears at hand a woolly sheep is shorn instead of killed.
pub fn choose(item: &str, entities: &[Entity], me: Vec3, shears: bool, mem: &Memory) -> Choice {
    let Some(y) = row(item) else {
        return Choice::NotFromAnimals;
    };
    let wool = y.shear;
    let mut seen: Vec<&Entity> = entities
        .iter()
        .filter(|e| y.from.contains(&e.kind) && !e.state.dying)
        .filter(|e| e.pos.dist(me) <= SOURCE_RADIUS)
        .collect();
    seen.sort_by(|a, b| a.pos.dist(me).total_cmp(&b.pos.dist(me)));
    let pick = seen
        .iter()
        .find(|e| !e.state.baby && !(wool && e.state.sheared) && !mem.avoid.contains(&e.id));
    if let Some(e) = pick {
        let method = if wool && shears && !mem.no_shear.contains(&e.id) {
            Method::Shear
        } else {
            Method::Kill
        };
        return Choice::Take { id: e.id, method };
    }
    let names = names(y.from);
    let r = SOURCE_RADIUS;
    Choice::NoneInRange(if seen.is_empty() {
        format!("no {names} within {r:.0} blocks to hunt")
    } else {
        let what = if wool {
            "babies, sheared or unreachable"
        } else {
            "babies or unreachable"
        };
        format!(
            "the {} {names} within {r:.0} blocks are all {what}",
            seen.len()
        )
    })
}

fn names(kinds: &[EntityKind]) -> String {
    kinds
        .iter()
        .map(|k| k.label())
        .collect::<Vec<_>>()
        .join(" or ")
}
