//! Ranged threats: arrows in flight (R14) and the archer stance (D3), with the archer's
//! draw and reload cadence and the line-of-sight exposure the stance and eating read.

mod cadence;
mod evade;
mod moves;
mod stance;

pub use cadence::{exposed_to, safe_for, ticks_to_hit, ticks_to_shot};
pub use evade::{ProjectileEvade, SHIELD_RAISE_TICKS};
pub use stance::RangedCover;

use crate::behaviour::Ctx;
use crate::snapshot::ItemKind;

fn shield_in_offhand(ctx: &Ctx) -> bool {
    ctx.snap
        .me
        .offhand
        .is_some_and(|s| s.kind == ItemKind::Shield)
}
