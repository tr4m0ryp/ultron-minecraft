//! Serde mirrors of the reflex-core knob types, converted at the bridge.

use reflex_core::knobs::{GainName, Mode, Response, Trigger};
use reflex_core::math::Vec3;
use serde::{Deserialize, Serialize};

use super::entity::parse_entity;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireMode {
    Explore,
    Mine,
    Build,
    Fight,
    Retreat,
    Follow,
    IdleSafe,
}

impl From<WireMode> for Mode {
    fn from(m: WireMode) -> Self {
        match m {
            WireMode::Explore => Mode::Explore,
            WireMode::Mine => Mode::Mine,
            WireMode::Build => Mode::Build,
            WireMode::Fight => Mode::Fight,
            WireMode::Retreat => Mode::Retreat,
            WireMode::Follow => Mode::Follow,
            WireMode::IdleSafe => Mode::IdleSafe,
        }
    }
}

pub fn mode_label(m: Mode) -> &'static str {
    match m {
        Mode::Explore => "explore",
        Mode::Mine => "mine",
        Mode::Build => "build",
        Mode::Fight => "fight",
        Mode::Retreat => "retreat",
        Mode::Follow => "follow",
        Mode::IdleSafe => "idle_safe",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireGain {
    Aggression,
    Caution,
    HpFloor,
    HungerFloor,
    Leash,
    Vigilance,
    AirFloor,
}

impl From<WireGain> for GainName {
    fn from(g: WireGain) -> Self {
        match g {
            WireGain::Aggression => GainName::Aggression,
            WireGain::Caution => GainName::Caution,
            WireGain::HpFloor => GainName::HpFloor,
            WireGain::HungerFloor => GainName::HungerFloor,
            WireGain::Leash => GainName::Leash,
            WireGain::Vigilance => GainName::Vigilance,
            WireGain::AirFloor => GainName::AirFloor,
        }
    }
}

/// Where an inhibition applies. Converted at the bridge, which knows position and goal.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WireScope {
    #[default]
    Everywhere,
    Region {
        min: [f64; 3],
        max: [f64; 3],
    },
    /// A box of half-width `radius` around the player's current position.
    Here {
        radius: f64,
    },
    UntilGoalEnds,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WireTrigger {
    EntityWithin {
        entity: String,
        dist: f64,
        #[serde(default)]
        of_companion: bool,
    },
    HpBelow {
        hp: f32,
    },
    CompanionHpBelow {
        hp: f32,
    },
    ChatMatches {
        pattern: String,
    },
}

impl TryFrom<WireTrigger> for Trigger {
    type Error = String;
    fn try_from(t: WireTrigger) -> Result<Self, String> {
        Ok(match t {
            WireTrigger::EntityWithin {
                entity,
                dist,
                of_companion,
            } => Trigger::EntityWithin {
                kind: parse_entity(&entity)?,
                dist: positive(dist, "dist")?,
                of_companion,
            },
            WireTrigger::HpBelow { hp } => Trigger::HpBelow(hp),
            WireTrigger::CompanionHpBelow { hp } => Trigger::CompanionHpBelow(hp),
            WireTrigger::ChatMatches { pattern } if pattern.trim().is_empty() => {
                return Err("chat_matches needs a non-empty pattern".into())
            }
            WireTrigger::ChatMatches { pattern } => Trigger::ChatMatches(pattern),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WireResponse {
    Say { text: String },
    RetreatTo { pos: [f64; 3] },
    FleeFrom { entity: String },
    Attack { entity: String },
    PlaceBlockBetween { entity: String },
}

impl TryFrom<WireResponse> for Response {
    type Error = String;
    fn try_from(r: WireResponse) -> Result<Self, String> {
        Ok(match r {
            WireResponse::Say { text } => Response::Say(super::chat_line(&text)?),
            WireResponse::RetreatTo { pos: [x, y, z] } => Response::RetreatTo(Vec3::new(x, y, z)),
            WireResponse::FleeFrom { entity } => Response::FleeFrom(parse_entity(&entity)?),
            WireResponse::Attack { entity } => Response::Attack(parse_entity(&entity)?),
            WireResponse::PlaceBlockBetween { entity } => {
                Response::PlaceBlockBetween(parse_entity(&entity)?)
            }
        })
    }
}

fn positive(v: f64, name: &str) -> Result<f64, String> {
    if v.is_finite() && v > 0.0 {
        Ok(v)
    } else {
        Err(format!("{name} must be a positive number"))
    }
}
