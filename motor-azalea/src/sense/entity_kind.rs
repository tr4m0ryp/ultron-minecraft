use azalea::registry::builtin::EntityKind as Az;
use reflex_core::snapshot::EntityKind;

/// Kinds the reflexes have no rule for map to `Other` (including hostiles such as blazes).
pub fn map_kind(kind: Az) -> EntityKind {
    use EntityKind as K;
    match kind {
        Az::Creeper => K::Creeper,
        Az::Zombie | Az::ZombieVillager => K::Zombie,
        Az::Husk => K::Husk,
        Az::Drowned => K::Drowned,
        Az::Skeleton => K::Skeleton,
        Az::Stray => K::Stray,
        Az::Bogged => K::Bogged,
        Az::Spider => K::Spider,
        Az::CaveSpider => K::CaveSpider,
        Az::Enderman => K::Enderman,
        Az::Witch => K::Witch,
        Az::Arrow | Az::SpectralArrow => K::Arrow,
        Az::Trident => K::Trident,
        Az::Fireball | Az::SmallFireball | Az::DragonFireball => K::Fireball,
        Az::FallingBlock => K::FallingBlock,
        Az::Item => K::Item,
        Az::Player => K::Player,
        Az::Wolf => K::Wolf,
        Az::IronGolem => K::IronGolem,
        Az::Villager | Az::WanderingTrader => K::Villager,
        Az::Bee => K::Bee,
        Az::ZombifiedPiglin => K::ZombifiedPiglin,
        Az::Cow
        | Az::Mooshroom
        | Az::Pig
        | Az::Sheep
        | Az::Chicken
        | Az::Rabbit
        | Az::Horse
        | Az::Donkey
        | Az::Mule
        | Az::Llama
        | Az::Cat
        | Az::Goat
        | Az::Turtle
        | Az::Parrot => K::Passive,
        _ => K::Other,
    }
}
