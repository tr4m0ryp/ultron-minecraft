//! Wires the goal layer's ports to Azalea: the registration hook for the executor.

use azalea::Client;
use reflex_core::{
    goals::{self, GoalHandle},
    Behaviour,
};

use crate::{nav::AzaleaNav, world::AzaleaWorld};

/// The behaviour catalog with the goal runner on Azalea's planner and world, plus the
/// handle the tick loop settles after each tick (`handle.settle(&mut exec.knobs)`).
pub fn goal_catalog(bot: &Client) -> (Vec<Box<dyn Behaviour>>, GoalHandle) {
    let handle = GoalHandle::with_ledger(crate::ledger::open());
    let behaviours = goals::catalog(
        Box::new(AzaleaNav::new(bot.clone())),
        Box::new(AzaleaWorld::new(bot.clone())),
        handle.clone(),
    );
    (behaviours, handle)
}
