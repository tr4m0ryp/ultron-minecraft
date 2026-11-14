use crate::math::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntityKind {
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
    Passive,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntityClass {
    Hostile,
    /// Must never be attacked by default (D11 mask).
    Protected,
    Passive,
    Projectile,
    FallingBlock,
    Item,
    Other,
}

impl EntityKind {
    pub fn class(self) -> EntityClass {
        use EntityKind::*;
        match self {
            Creeper | Zombie | Husk | Drowned | Skeleton | Stray | Bogged | Spider | CaveSpider
            | Witch => EntityClass::Hostile,
            Enderman | Player | Wolf | IronGolem | Villager | Bee | ZombifiedPiglin => {
                EntityClass::Protected
            }
            Passive => EntityClass::Passive,
            Arrow | Trident | Fireball => EntityClass::Projectile,
            FallingBlock => EntityClass::FallingBlock,
            Item => EntityClass::Item,
            Other => EntityClass::Other,
        }
    }

    pub fn is_melee(self) -> bool {
        use EntityKind::*;
        matches!(self, Zombie | Husk | Drowned | Spider | CaveSpider)
    }

    pub fn is_ranged(self) -> bool {
        matches!(self, EntityKind::Skeleton | EntityKind::Stray | EntityKind::Bogged)
    }

    pub fn eye_height(self) -> f64 {
        match self {
            EntityKind::Enderman => 2.55,
            EntityKind::Spider | EntityKind::CaveSpider => 0.65,
            EntityKind::Arrow | EntityKind::Trident | EntityKind::Fireball | EntityKind::Item => 0.1,
            _ => 1.62,
        }
    }

    pub fn label(self) -> &'static str {
        use EntityKind::*;
        match self {
            Creeper => "creeper",
            Zombie => "zombie",
            Husk => "husk",
            Drowned => "drowned",
            Skeleton => "skeleton",
            Stray => "stray",
            Bogged => "bogged",
            Spider => "spider",
            CaveSpider => "cave_spider",
            Enderman => "enderman",
            Witch => "witch",
            Arrow => "arrow",
            Trident => "trident",
            Fireball => "fireball",
            FallingBlock => "falling_block",
            Item => "item",
            Player => "player",
            Wolf => "wolf",
            IronGolem => "iron_golem",
            Villager => "villager",
            Bee => "bee",
            ZombifiedPiglin => "zombified_piglin",
            Passive => "passive",
            Other => "other",
        }
    }
}

/// Metadata the reflexes read. Fields that do not apply to a kind stay false.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EntityState {
    pub fuse_lit: bool,
    pub drawing_bow: bool,
    pub baby: bool,
    pub holding_trident: bool,
    /// Projectile already stuck in a block or entity.
    pub inert: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entity {
    pub id: u32,
    pub kind: EntityKind,
    pub pos: Vec3,
    pub vel: Vec3,
    pub state: EntityState,
    pub name: Option<String>,
}

impl Entity {
    pub fn new(id: u32, kind: EntityKind, pos: Vec3) -> Self {
        Self { id, kind, pos, vel: Vec3::ZERO, state: EntityState::default(), name: None }
    }

    pub fn head(&self) -> Vec3 {
        self.pos + Vec3::new(0.0, self.kind.eye_height(), 0.0)
    }
}
