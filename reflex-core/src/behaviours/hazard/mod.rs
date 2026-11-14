//! Level 1 protective reflexes (spinal and brainstem analogues).

mod air;
mod contact;
mod edge;
mod extinguish;
mod falling;
mod lava;

pub use air::SurfaceForAir;
pub use contact::ContactHazard;
pub use edge::{probe_drop, Drop, EdgeGuard, VoidGuard};
pub use extinguish::Extinguish;
pub use falling::FallingBlocks;
pub use lava::LavaWithdrawal;
mod dig_guard;
pub use dig_guard::DigGuard;
