//! The bot's own state: pose, vitals, effects, hands and inventory.

use azalea::{
    attack::AttackStrengthScale,
    block::fluid_state::FluidKind,
    ecs::{entity::Entity, world::World},
    entity::{
        ActiveEffects, FluidOnEyes, LookDirection, Physics, Position,
        inventory::Inventory,
        metadata::{AbstractLivingUsingItem, AirSupply, Health, OnFire, PlayerAbsorption, Sprinting},
    },
    local_player::Hunger,
    registry::builtin::{ItemKind as AzItem, MobEffect},
};
use reflex_core::{math::Vec3, snapshot::SelfState};

use super::items::map_stack;

/// Player inventory menu layout: 0 craft result, 1..=4 craft, 5..=8 armor (5 = head),
/// 9..=35 main, 36..=44 hotbar, 45 offhand.
const MENU_HEAD: usize = 5;
const MENU_MAIN: std::ops::RangeInclusive<usize> = 9..=35;
const MENU_HOTBAR: usize = 36;
const MENU_OFFHAND: usize = 45;

pub fn vec3(v: azalea::Vec3) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

pub fn read(ecs: &World, me: Entity) -> Option<SelfState> {
    let pos = vec3(**ecs.get::<Position>(me)?);
    let physics = ecs.get::<Physics>(me)?;
    let look = ecs.get::<LookDirection>(me)?;
    let mut s = SelfState {
        pos,
        vel: vec3(physics.velocity),
        yaw: look.y_rot(),
        pitch: look.x_rot(),
        on_ground: physics.on_ground(),
        in_water: physics.is_in_water(),
        fall_distance: physics.fall_distance as f32,
        ..SelfState::default()
    };
    if let Some(h) = ecs.get::<Health>(me) {
        s.health = **h;
    }
    if let Some(a) = ecs.get::<PlayerAbsorption>(me) {
        s.absorption = **a;
    }
    if let Some(h) = ecs.get::<Hunger>(me) {
        s.food = h.food.min(255) as u8;
        s.saturation = h.saturation;
    }
    if let Some(a) = ecs.get::<AirSupply>(me) {
        s.air = **a;
    }
    s.on_fire = ecs.get::<OnFire>(me).is_some_and(|f| **f);
    s.head_in_water = ecs.get::<FluidOnEyes>(me).is_some_and(|f| **f == FluidKind::Water);
    s.sprinting = ecs.get::<Sprinting>(me).is_some_and(|f| **f);
    s.using_item = ecs.get::<AbstractLivingUsingItem>(me).is_some_and(|f| **f);
    if let Some(e) = ecs.get::<ActiveEffects>(me) {
        s.fire_resistance = e.0.contains_key(&MobEffect::FireResistance);
        s.water_breathing = e.0.contains_key(&MobEffect::WaterBreathing);
    }
    if let Some(a) = ecs.get::<AttackStrengthScale>(me) {
        s.attack_ready = **a;
    }
    // Azalea does not track a sneaking flag separate from the requested one; the request
    // is applied on the next client tick, so this lags the real pose by one tick.
    s.sneaking = ecs
        .get::<azalea::ClientMovementState>(me)
        .is_some_and(|m| m.trying_to_crouch);
    if let Some(inv) = ecs.get::<Inventory>(me) {
        read_inventory(inv, &mut s);
    }
    Some(s)
}

fn read_inventory(inv: &Inventory, s: &mut SelfState) {
    let menu = &inv.inventory_menu;
    s.selected_slot = inv.selected_hotbar_slot;
    for (i, slot) in s.hotbar.iter_mut().enumerate() {
        *slot = menu.slot(MENU_HOTBAR + i).and_then(map_stack);
    }
    s.offhand = menu.slot(MENU_OFFHAND).and_then(map_stack);
    for idx in MENU_MAIN {
        if let Some(st) = menu.slot(idx).and_then(map_stack) {
            s.inventory.push((idx as u8, st));
        }
    }
    s.wearing_pumpkin = menu
        .slot(MENU_HEAD)
        .and_then(|st| st.as_present())
        .is_some_and(|d| d.kind == AzItem::CarvedPumpkin);
}

/// Hotbar 0..=8, main 9..=35 and offhand 40 (reflex slot numbers) to player-menu indices.
pub fn menu_index(slot: u8) -> Option<u16> {
    match slot {
        0..=8 => Some(MENU_HOTBAR as u16 + slot as u16),
        9..=35 => Some(slot as u16),
        40 => Some(MENU_OFFHAND as u16),
        _ => None,
    }
}
