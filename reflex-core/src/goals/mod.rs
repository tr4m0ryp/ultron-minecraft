//! The goal layer: the brain's goals as resumable state machines that bid at P5 through the
//! ordinary selector, so reflexes pre-empt them and they resume afterwards. Navigation and
//! world knowledge sit behind the `ports` traits, keeping this module library-agnostic.

pub mod care;
pub mod combat;
pub mod fakes;
pub mod gather;
pub mod items;
pub mod ledger;
pub mod ports;
pub mod runner;
pub mod spec;
pub mod task;
pub mod travel;

pub use ledger::{Keep, Ledger};
pub use ports::{
    kind_matches, ItemDrop, MenuStatus, NavFailure, NavStatus, NavTarget, Navigator, Recipe, Steer,
    ToolNeed, Work, World,
};
pub use runner::{GoalHandle, GoalRunner, GoalState, GoalStatus, Reporter};
pub use spec::{parse, AttackTarget, GoalSpec, Person};
pub use task::{GoalCx, GoalTask, Intent, Plan, Step};

use crate::behaviour::Behaviour;
use crate::behaviours::{default_catalog, GOAL_ID};

/// The state machine for a parsed spec.
pub fn build(spec: &GoalSpec) -> Box<dyn GoalTask> {
    match spec.clone() {
        GoalSpec::Follow { who, distance } => Box::new(travel::Follow::new(who, distance)),
        GoalSpec::GoTo(p) => Box::new(travel::GoTo::new(p)),
        GoalSpec::ComeHere => Box::new(travel::ComeHere::new()),
        GoalSpec::Stay => Box::new(travel::Stay::default()),
        GoalSpec::Mine { block, count } => Box::new(gather::Mine::new(block, count)),
        GoalSpec::Collect { item, count } => Box::new(gather::Collect::new(item, count)),
        GoalSpec::Attack(t) => Box::new(combat::Attack::new(t)),
        GoalSpec::Guard(who) => Box::new(combat::Guard::new(who)),
        GoalSpec::Craft { item, count } => Box::new(items::Craft::new(item, count)),
        GoalSpec::Give { who, item, count } => Box::new(items::Give::new(who, item, count)),
        GoalSpec::Place {
            block,
            pos,
            to: None,
            replace,
        } => Box::new(items::Place::new(block, pos).replacing(replace)),
        GoalSpec::Place {
            block,
            pos,
            to: Some(to),
            replace,
        } => Box::new(items::Fill::new(block, pos, to).replacing(replace)),
        GoalSpec::Break(pos) => Box::new(items::BreakAt::new(pos)),
        GoalSpec::SleepInBed => Box::new(care::SleepInBed::new()),
        GoalSpec::EatNow => Box::new(care::EatNow::default()),
    }
}

/// Registration hook: the default behaviour catalog with the straight-line goal walker
/// replaced by the goal runner, in the same slot so veto order is unchanged.
pub fn catalog(
    nav: Box<dyn Navigator>,
    world: Box<dyn World>,
    handle: GoalHandle,
) -> Vec<Box<dyn Behaviour>> {
    let mut runner: Option<Box<dyn Behaviour>> =
        Some(Box::new(GoalRunner::new(nav, world, handle)));
    default_catalog()
        .into_iter()
        .map(|b| {
            if b.id() == GOAL_ID {
                runner.take().unwrap_or(b)
            } else {
                b
            }
        })
        .collect()
}
