//! Entity kinds by their snake_case label; reflex-core has `label()` but no parser.

use reflex_core::snapshot::EntityKind;

pub const ALL_KINDS: [EntityKind; 29] = {
    use EntityKind::*;
    [
        Creeper,
        Zombie,
        Husk,
        Drowned,
        Skeleton,
        Stray,
        Bogged,
        Spider,
        CaveSpider,
        Enderman,
        Witch,
        Arrow,
        Trident,
        Fireball,
        FallingBlock,
        Item,
        Player,
        Wolf,
        IronGolem,
        Villager,
        Bee,
        ZombifiedPiglin,
        Sheep,
        Cow,
        Pig,
        Chicken,
        Rabbit,
        Passive,
        Other,
    ]
};

/// Exhaustive on purpose: a new upstream variant breaks the build here until it is added
/// to `ALL_KINDS`.
#[allow(dead_code)]
fn exhaustive(k: EntityKind) {
    use EntityKind::*;
    match k {
        Creeper | Zombie | Husk | Drowned | Skeleton | Stray | Bogged | Spider | CaveSpider
        | Enderman | Witch | Arrow | Trident | Fireball | FallingBlock | Item | Player | Wolf
        | IronGolem | Villager | Bee | ZombifiedPiglin | Sheep | Cow | Pig | Chicken | Rabbit
        | Passive | Other => {}
    }
}

pub fn parse_entity(s: &str) -> Result<EntityKind, String> {
    let s = s.trim().to_ascii_lowercase().replace([' ', '-'], "_");
    ALL_KINDS
        .iter()
        .copied()
        .find(|k| k.label() == s)
        .ok_or_else(|| format!("unknown entity kind '{s}'"))
}

pub fn entity_labels() -> Vec<&'static str> {
    ALL_KINDS.iter().map(|k| k.label()).collect()
}
