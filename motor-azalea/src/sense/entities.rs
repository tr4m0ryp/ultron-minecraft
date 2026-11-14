//! Nearby entities, their metadata-derived state, and the companion player.

use std::collections::HashMap;

use azalea::{
    core::entity_id::MinecraftEntityId,
    ecs::{entity::Entity, query::Without, world::World},
    entity::{
        metadata::{
            AbstractAgeableBaby, AbstractLivingUsingItem, Health, IsIgnited, SwellDir, ZombieBaby,
        },
        EntityKindComponent, EntityUuid, LocalEntity, Physics, Position,
    },
    local_player::TabList,
    world::WorldName,
};
use reflex_core::{
    math::Vec3,
    snapshot::{Companion, Entity as SnapEntity, EntityKind, EntityState},
};

use super::{body::vec3, entity_kind::map_kind};

pub const RADIUS: f64 = 32.0;
/// A projectile moving slower than this (blocks per tick) is treated as stuck in place.
/// Heuristic: Azalea exposes no in-ground flag for arrows.
const INERT_SPEED: f64 = 0.02;

pub struct EntityView {
    pub entities: Vec<SnapEntity>,
    pub companion: Option<Companion>,
}

/// `prev` maps entity id to its position on the previous tick; it is replaced with this
/// tick's positions. Velocity is the position delta since the previous tick (see
/// `velocity`).
pub fn read(
    ecs: &mut World,
    me: Entity,
    my_pos: Vec3,
    prev: &mut HashMap<i32, Vec3>,
    companion_name: Option<&str>,
) -> Option<EntityView> {
    let world_name = ecs.get::<WorldName>(me)?.clone();
    let tab: HashMap<_, _> = ecs
        .get::<TabList>(me)
        .map(|t| {
            t.iter()
                .map(|(u, p)| (*u, p.profile.name.clone()))
                .collect()
        })
        .unwrap_or_default();

    let mut query = ecs.query_filtered::<(
        Entity,
        &MinecraftEntityId,
        &EntityKindComponent,
        &Position,
        &WorldName,
        &EntityUuid,
    ), Without<LocalEntity>>();
    let candidates: Vec<_> = query
        .iter(ecs)
        .filter(|(.., w, _)| **w == world_name)
        .map(|(e, id, k, p, _, u)| (e, id.0, k.0, vec3(**p), **u))
        .collect();

    let mut now = HashMap::with_capacity(candidates.len());
    let mut entities = Vec::with_capacity(candidates.len());
    let mut companion = None;
    for (e, id, kind, pos, uuid) in candidates {
        // The companion is tracked as far as the server sends it; everything else only
        // within the sensing radius.
        let far = pos.dist(my_pos) > RADIUS;
        if far && !(kind == azalea::registry::builtin::EntityKind::Player && companion.is_none()) {
            continue;
        }
        let delta = prev.get(&id).map(|old| pos - *old);
        let vel = velocity(ecs, e, delta);
        now.insert(id, pos);
        let kind_s = map_kind(kind);
        let mut ent = SnapEntity::new(id as u32, kind_s, pos);
        ent.vel = vel;
        ent.state = state_of(ecs, e, kind_s, delta.is_some() && vel.len() < INERT_SPEED);
        if kind_s == EntityKind::Player {
            ent.name = tab.get(&uuid).cloned();
            if companion.is_none()
                && companion_name.is_some()
                && ent.name.as_deref() == companion_name
            {
                companion = Some(Companion {
                    name: ent.name.clone().unwrap_or_default(),
                    entity_id: Some(ent.id),
                    pos,
                    vel,
                    health: ecs.get::<Health>(e).map(|h| **h),
                });
            }
        }
        if far {
            continue;
        }
        entities.push(ent);
    }
    *prev = now;
    Some(EntityView {
        entities,
        companion,
    })
}

/// Azalea runs physics only for the local player, so a remote entity's `Physics::velocity`
/// is the last motion packet (spawn or knockback) and never decays. Prefer the observed
/// position delta; fall back to the packet velocity on first sighting (arrows, tnt).
fn velocity(ecs: &World, e: Entity, delta: Option<Vec3>) -> Vec3 {
    if let Some(d) = delta {
        return d;
    }
    ecs.get::<Physics>(e)
        .map_or(Vec3::ZERO, |p| vec3(p.velocity))
}

fn state_of(ecs: &World, e: Entity, kind: EntityKind, stationary: bool) -> EntityState {
    let mut st = EntityState::default();
    match kind {
        EntityKind::Creeper => {
            let swelling = ecs.get::<SwellDir>(e).is_some_and(|s| **s > 0);
            let ignited = ecs.get::<IsIgnited>(e).is_some_and(|i| **i);
            st.fuse_lit = swelling || ignited;
        }
        EntityKind::Skeleton | EntityKind::Stray | EntityKind::Bogged => {
            st.drawing_bow = ecs.get::<AbstractLivingUsingItem>(e).is_some_and(|u| **u);
        }
        EntityKind::Zombie | EntityKind::Husk | EntityKind::Drowned => {
            st.baby = ecs.get::<ZombieBaby>(e).is_some_and(|b| **b);
        }
        EntityKind::Villager | EntityKind::Passive | EntityKind::Wolf | EntityKind::Bee => {
            st.baby = ecs.get::<AbstractAgeableBaby>(e).is_some_and(|b| **b);
        }
        EntityKind::Arrow | EntityKind::Trident => {
            st.inert = stationary;
        }
        _ => {}
    }
    // holding_trident is not populated: equipment of remote entities is not exposed.
    st
}
