use crate::math::Vec3;
use crate::snapshot::EntityKind;

/// Fixed trigger vocabulary for brain-armed responses (subset of the design note, 4.6).
#[derive(Clone, Debug, PartialEq)]
pub enum Trigger {
    /// An entity of `kind` within `dist` of us, or of the companion when `of_companion`.
    EntityWithin {
        kind: EntityKind,
        dist: f64,
        of_companion: bool,
    },
    HpBelow(f32),
    CompanionHpBelow(f32),
    /// Case-insensitive substring match on any chat line.
    ChatMatches(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Response {
    Say(String),
    RetreatTo(Vec3),
    FleeFrom(EntityKind),
    Attack(EntityKind),
    /// Place a throwaway block on the ground between the nearest `threat` and the companion.
    PlaceBlockBetween(EntityKind),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArmedResponse {
    pub id: u64,
    pub trigger: Trigger,
    pub response: Response,
    pub expires_at: Option<u64>,
    pub once: bool,
}
