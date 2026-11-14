//! Compact per-tick world snapshot. Everything a behaviour may read lives here; the motor
//! adapter fills it from the client library each tick.

mod body;
mod entity;
mod events;
mod items;
mod world;

pub use body::SelfState;
pub use entity::{Entity, EntityClass, EntityKind, EntityState};
pub use events::{ChatEvent, Companion, DamageEvent, SoundEvent, SoundKind};
pub use items::{ItemKind, ItemStack, Slot};
pub use world::{BlockKind, LocalBlocks};

use crate::math::Vec3;

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub tick: u64,
    pub me: SelfState,
    pub blocks: LocalBlocks,
    pub entities: Vec<Entity>,
    pub companion: Option<Companion>,
    /// Chat, damage and sound events received since the previous snapshot.
    pub chat: Vec<ChatEvent>,
    pub damage: Vec<DamageEvent>,
    pub sounds: Vec<SoundEvent>,
    /// Lowest buildable y of the dimension; below it is the void.
    pub world_min_y: i32,
    pub is_night: bool,
    /// The configured companion's name, known even while the server does not track them.
    pub companion_name: Option<String>,
}

impl Snapshot {
    pub fn eye(&self) -> Vec3 {
        self.me.eye()
    }

    pub fn entity(&self, id: u32) -> Option<&Entity> {
        self.entities.iter().find(|e| e.id == id)
    }

    /// Entities of a class within `radius` blocks, nearest first.
    pub fn nearby(&self, radius: f64, pred: impl Fn(&Entity) -> bool) -> Vec<&Entity> {
        let mut v: Vec<&Entity> = self
            .entities
            .iter()
            .filter(|e| e.pos.dist(self.me.pos) <= radius && pred(e))
            .collect();
        v.sort_by(|a, b| a.pos.dist(self.me.pos).total_cmp(&b.pos.dist(self.me.pos)));
        v
    }

    pub fn nearest_hostile(&self, radius: f64) -> Option<&Entity> {
        self.nearby(radius, |e| e.kind.class() == EntityClass::Hostile)
            .into_iter()
            .next()
    }
}
