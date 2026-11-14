use crate::math::Vec3;

#[derive(Clone, Debug, PartialEq)]
pub struct ChatEvent {
    pub sender: String,
    pub text: String,
    /// Sender's position if it is a player we can see.
    pub sender_pos: Option<Vec3>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DamageEvent {
    pub amount: f32,
    pub source_pos: Option<Vec3>,
    pub attacker: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SoundKind {
    CreeperPrimed,
    TntPrimed,
    BowDraw,
    Explosion,
    HostileStep,
    PlayerHurt,
    Other,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SoundEvent {
    pub kind: SoundKind,
    pub pos: Vec3,
}

/// The human player the bot plays alongside.
#[derive(Clone, Debug, PartialEq)]
pub struct Companion {
    pub name: String,
    pub entity_id: Option<u32>,
    pub pos: Vec3,
    pub vel: Vec3,
    /// Health if the server exposes it; `None` when unknown.
    pub health: Option<f32>,
}
