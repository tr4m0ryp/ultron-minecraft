use super::items::{ItemKind, ItemStack, Slot};
use crate::math::Vec3;

pub const MAX_AIR: i32 = 300;
pub const EYE_HEIGHT: f64 = 1.62;

#[derive(Clone, Debug)]
pub struct SelfState {
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub health: f32,
    pub absorption: f32,
    pub food: u8,
    pub saturation: f32,
    /// Remaining air in ticks, 0..=300.
    pub air: i32,
    pub on_ground: bool,
    pub on_fire: bool,
    pub in_water: bool,
    pub head_in_water: bool,
    pub fire_resistance: bool,
    pub water_breathing: bool,
    pub fall_distance: f32,
    pub sneaking: bool,
    pub sprinting: bool,
    /// True while eating, drinking, drawing a bow or raising a shield.
    pub using_item: bool,
    /// Ticks the shield has been held up (the adapter's efference copy), 0 when lowered.
    /// A raised shield blocks only from its 5th tick.
    pub shield_ticks: u32,
    pub wearing_pumpkin: bool,
    /// Attack cooldown progress in 0..=1; 1 means a full-strength swing is ready.
    pub attack_ready: f32,
    pub selected_slot: Slot,
    pub hotbar: [Option<ItemStack>; 9],
    pub offhand: Option<ItemStack>,
    /// Main inventory slots 9..=35 that hold something.
    pub inventory: Vec<(Slot, ItemStack)>,
}

impl Default for SelfState {
    fn default() -> Self {
        Self {
            pos: Vec3::ZERO,
            vel: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            health: 20.0,
            absorption: 0.0,
            food: 20,
            saturation: 5.0,
            air: MAX_AIR,
            on_ground: true,
            on_fire: false,
            in_water: false,
            head_in_water: false,
            fire_resistance: false,
            water_breathing: false,
            fall_distance: 0.0,
            sneaking: false,
            sprinting: false,
            shield_ticks: 0,
            using_item: false,
            wearing_pumpkin: false,
            attack_ready: 1.0,
            selected_slot: 0,
            hotbar: [None; 9],
            offhand: None,
            inventory: Vec::new(),
        }
    }
}

impl SelfState {
    pub fn eye(&self) -> Vec3 {
        self.pos + Vec3::new(0.0, EYE_HEIGHT, 0.0)
    }

    pub fn held(&self) -> Option<ItemStack> {
        self.hotbar
            .get(self.selected_slot as usize)
            .copied()
            .flatten()
    }

    pub fn hp_total(&self) -> f32 {
        self.health + self.absorption
    }

    /// First hotbar slot holding an item matching `pred`.
    pub fn hotbar_find(&self, pred: impl Fn(ItemKind) -> bool) -> Option<Slot> {
        self.hotbar
            .iter()
            .position(|s| s.is_some_and(|s| pred(s.kind)))
            .map(|i| i as Slot)
    }

    /// Any slot (hotbar first, then main inventory) holding an item matching `pred`.
    pub fn find_any(&self, pred: impl Fn(ItemKind) -> bool + Copy) -> Option<Slot> {
        self.hotbar_find(pred).or_else(|| {
            self.inventory
                .iter()
                .find(|(_, s)| pred(s.kind))
                .map(|(i, _)| *i)
        })
    }

    pub fn has(&self, pred: impl Fn(ItemKind) -> bool + Copy) -> bool {
        self.find_any(pred).is_some() || self.offhand.is_some_and(|s| pred(s.kind))
    }
}
