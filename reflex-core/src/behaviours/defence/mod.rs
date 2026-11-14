//! Level 2 defensive stances by mob type (PAG and amygdala analogue), the melee and shield
//! skills they use, and the gaze and attack filters.

mod creeper;
mod filters;
mod melee;
mod ranged;
mod totem;

pub use creeper::CreeperStance;
pub use filters::{AttackMask, EndermanGazeFilter};
pub use melee::{footing, melee_commands, MeleeDefence};
pub use ranged::{
    exposed_to, safe_for, ticks_to_hit, ticks_to_shot, ProjectileEvade, RangedCover,
    SHIELD_RAISE_TICKS,
};
pub use totem::Totem;
