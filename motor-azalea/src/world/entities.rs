//! Item drops on the ground and the sleeping flag, read from the ECS.

use azalea::{
    core::entity_id::MinecraftEntityId,
    entity::{
        metadata::{ItemItem, SleepingPos},
        Position,
    },
    world::WorldName,
    Client,
};
use reflex_core::{goals::ItemDrop, math::Vec3};

use super::names::{item_matches, item_name};

pub fn drops(bot: &Client, item: Option<&str>, around: Vec3, radius: f64) -> Vec<ItemDrop> {
    let mut ecs = bot.ecs.write();
    let Some(mine) = ecs.get::<WorldName>(bot.entity).cloned() else {
        return Vec::new();
    };
    let mut q = ecs.query::<(&MinecraftEntityId, &ItemItem, &Position, &WorldName)>();
    q.iter(&ecs)
        .filter(|(_, _, _, w)| **w == mine)
        .filter(|(_, it, _, _)| item.is_none_or(|k| item_matches(k, &it.0)))
        .filter_map(|(id, it, p, _)| {
            let pos = Vec3::new(p.x, p.y, p.z);
            (pos.dist(around) <= radius).then(|| ItemDrop {
                entity_id: id.0 as u32,
                item: item_name(&it.0).unwrap_or("air").to_string(),
                count: it.0.count().max(0) as u32,
                pos,
            })
        })
        .collect()
}

pub fn sleeping(bot: &Client) -> bool {
    let ecs = bot.ecs.read();
    ecs.get::<SleepingPos>(bot.entity)
        .is_some_and(|s| s.0.is_some())
}
