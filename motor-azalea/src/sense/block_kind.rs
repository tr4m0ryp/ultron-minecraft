//! Azalea block state -> the coarse classes the reflexes reason about.

use azalea::{
    block::{
        fluid_state::{FluidKind, FluidState},
        BlockState, BlockTrait,
    },
    physics::collision::BlockWithShape,
    registry::{builtin::BlockKind as Az, tags::blocks},
};
use reflex_core::snapshot::BlockKind;

pub fn classify(state: BlockState) -> BlockKind {
    if state.is_air() {
        return BlockKind::Air;
    }
    let kind = Az::from(state);
    match kind {
        Az::CaveAir | Az::VoidAir => return BlockKind::Air,
        Az::Lava => return BlockKind::Lava,
        Az::Water => return BlockKind::Water,
        Az::Fire | Az::SoulFire => return BlockKind::Fire,
        Az::Campfire | Az::SoulCampfire => {
            let lit = Box::<dyn BlockTrait>::from(state).get_property("lit") == Some("true");
            return if lit {
                BlockKind::Campfire
            } else {
                BlockKind::Solid
            };
        }
        Az::Cactus => return BlockKind::Cactus,
        Az::SweetBerryBush => return BlockKind::BerryBush,
        Az::WitherRose => return BlockKind::WitherRose,
        Az::MagmaBlock => return BlockKind::Magma,
        Az::PowderSnow => return BlockKind::PowderSnow,
        Az::SlimeBlock | Az::Cobweb | Az::HayBlock => return BlockKind::Cushion,
        Az::Sand | Az::RedSand | Az::Gravel => return BlockKind::Gravity,
        _ => {}
    }
    if blocks::CLIMBABLE.contains(&kind) {
        return BlockKind::Climbable;
    }
    if blocks::CONCRETE_POWDERS.contains(&kind) {
        return BlockKind::Gravity;
    }
    if state.is_collision_shape_empty() {
        // Kelp, seagrass, bubble columns: no collision but the cell holds fluid.
        return match FluidState::from(state).kind {
            FluidKind::Water => BlockKind::Water,
            FluidKind::Lava => BlockKind::Lava,
            FluidKind::Empty => BlockKind::Air,
        };
    }
    BlockKind::Solid
}
