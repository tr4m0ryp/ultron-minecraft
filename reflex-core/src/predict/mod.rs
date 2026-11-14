//! Per-tick forward model (the cerebellum): fall prediction, time to contact, closest
//! approach of projectiles, and the efference copy of our own last command.

mod contact;
mod efference;
mod fall;

pub use contact::{cpa, tau};
pub use efference::{expected_step, speed_per_tick, Efference};
pub use fall::{fall_damage, predict_fall, FallPrediction, DRAG, GRAVITY};

use crate::snapshot::Snapshot;

#[derive(Clone, Debug, Default)]
pub struct Prediction {
    pub fall: Option<FallPrediction>,
    pub efference: Efference,
}

impl Prediction {
    pub fn compute(snap: &Snapshot, efference: Efference) -> Self {
        Self {
            fall: predict_fall(snap),
            efference,
        }
    }
}
