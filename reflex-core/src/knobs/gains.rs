use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GainName {
    Aggression,
    Caution,
    HpFloor,
    HungerFloor,
    Leash,
    Vigilance,
    AirFloor,
}

/// Brain-set gains. Defaults follow the design note, section 4.3.
#[derive(Clone, Debug, PartialEq)]
pub struct Gains {
    pub aggression: f32,
    pub caution: f32,
    pub hp_floor: f32,
    pub hunger_floor: u8,
    /// Companion leash distance in blocks; 0 disables following.
    pub leash: f64,
    pub vigilance: f32,
    /// Air level in ticks below which R8 surfaces.
    pub air_floor: i32,
    /// Per-behaviour salience gain g_b from the mode, default 1.
    pub per_behaviour: HashMap<String, f32>,
}

impl Default for Gains {
    fn default() -> Self {
        Self {
            aggression: 0.5,
            caution: 0.5,
            hp_floor: 8.0,
            hunger_floor: 14,
            leash: 6.0,
            vigilance: 0.5,
            air_floor: 100,
            per_behaviour: HashMap::new(),
        }
    }
}

impl Gains {
    pub fn set(&mut self, name: GainName, v: f32) {
        match name {
            GainName::Aggression => self.aggression = v.clamp(0.0, 1.0),
            GainName::Caution => self.caution = v.clamp(0.0, 1.0),
            GainName::HpFloor => self.hp_floor = v.clamp(0.0, 20.0),
            GainName::HungerFloor => self.hunger_floor = v.clamp(0.0, 20.0) as u8,
            GainName::Leash => self.leash = v.max(0.0) as f64,
            GainName::Vigilance => self.vigilance = v.clamp(0.0, 1.0),
            GainName::AirFloor => self.air_floor = v.clamp(0.0, 300.0) as i32,
        }
    }

    pub fn behaviour_gain(&self, id: &str) -> f32 {
        self.per_behaviour.get(id).copied().unwrap_or(1.0)
    }

    /// Deepest drop R5 lets the bot walk off: 3 blocks at caution 0, 1 block at caution 1.
    pub fn safe_drop(&self) -> f64 {
        3.0 - 2.0 * self.caution as f64
    }
}
