use crate::channels::{LegsCmd, Speed};
use crate::math::Vec3;

/// Copy of the motor command sent last tick, so the next tick can tell self-caused motion
/// from external pushes and detect being stuck.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Efference {
    pub legs: Option<LegsCmd>,
    /// Position at the moment the command was sent.
    pub from: Vec3,
}

/// Approximate ground speed in blocks per tick for each gait.
pub fn speed_per_tick(s: Speed) -> f64 {
    match s {
        Speed::Stop => 0.0,
        Speed::Sneak => 0.065,
        Speed::Walk => 0.216,
        Speed::Sprint => 0.28,
    }
}

/// Horizontal displacement a legs command should produce in one tick.
pub fn expected_step(cmd: &LegsCmd) -> Vec3 {
    cmd.heading.horizontal().normalized() * speed_per_tick(cmd.speed)
}
