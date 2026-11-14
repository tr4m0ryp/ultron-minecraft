//! The week-one behaviour catalog (design note section 7, items 1 to 10).

pub mod common;
pub mod defence;
pub mod goal;
pub mod hazard;
pub mod homeostasis;
pub mod locomotion;
pub mod orient;

pub use goal::{GoalExecutor, GOAL_ID};

use crate::behaviour::Behaviour;

/// Every behaviour in build order. Vetoes run in this order too, so the gaze filter (D5)
/// and attack mask (D11) come after anything that might write gaze or attack.
pub fn default_catalog() -> Vec<Box<dyn Behaviour>> {
    vec![
        Box::new(hazard::LavaWithdrawal),
        Box::new(hazard::ContactHazard),
        Box::new(hazard::Extinguish::default()),
        Box::new(hazard::EdgeGuard::default()),
        Box::new(hazard::VoidGuard::default()),
        Box::new(hazard::FallArrest::default()),
        Box::new(hazard::KnockbackRecovery),
        Box::new(hazard::DigGuard::default()),
        Box::new(hazard::FallingBlocks),
        Box::new(hazard::SurfaceForAir),
        Box::new(homeostasis::Hunger::default()),
        Box::new(homeostasis::Health),
        Box::new(defence::CreeperStance),
        Box::new(orient::Looming::default()),
        Box::new(defence::MeleeDefence),
        Box::new(homeostasis::CompanionLeash::default()),
        Box::new(orient::LookAtSpeaker::default()),
        Box::new(locomotion::Unstuck::default()),
        Box::new(locomotion::Swim),
        Box::new(defence::Totem),
        Box::new(defence::ProjectileEvade),
        Box::new(defence::RangedCover),
        Box::new(GoalExecutor::default()),
        Box::new(defence::EndermanGazeFilter),
        Box::new(defence::AttackMask),
    ]
}
