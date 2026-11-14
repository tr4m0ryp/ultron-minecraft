//! The goal vocabulary and its parser. The brain sets goals through the existing knobs API
//! (`Knobs::set_goal(spec, priority, target, solo)`); the spec string names the goal.

mod parse;

pub use parse::parse;

use crate::math::{BlockPos, Vec3};
use crate::snapshot::EntityKind;

/// A player the goal is about: the companion, or another player by name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Person {
    Companion,
    Named(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttackTarget {
    Id(u32),
    Kind(EntityKind),
}

#[derive(Clone, Debug, PartialEq)]
pub enum GoalSpec {
    Follow {
        who: Person,
        distance: f64,
    },
    GoTo(Vec3),
    ComeHere,
    Stay,
    Mine {
        block: String,
        count: u32,
    },
    Collect {
        item: String,
        count: u32,
    },
    Attack(AttackTarget),
    Guard(Person),
    Craft {
        item: String,
        count: u32,
    },
    Give {
        who: Person,
        item: String,
        count: u32,
    },
    Place {
        block: String,
        pos: BlockPos,
    },
    SleepInBed,
    EatNow,
}

impl GoalSpec {
    /// Goals that bring the body to the companion themselves, with the path planner. The
    /// straight-line leash (H8) stands down for them: it would drag the body into walls.
    pub fn tracks_companion(&self) -> bool {
        use GoalSpec::*;
        matches!(
            self,
            Follow {
                who: Person::Companion,
                ..
            } | ComeHere
                | Guard(Person::Companion)
                | Give {
                    who: Person::Companion,
                    ..
                }
        )
    }

    /// Short label used as the subject of every status event.
    pub fn label(&self) -> &'static str {
        match self {
            GoalSpec::Follow { .. } => "follow",
            GoalSpec::GoTo(_) => "go_to",
            GoalSpec::ComeHere => "come_here",
            GoalSpec::Stay => "stay",
            GoalSpec::Mine { .. } => "mine",
            GoalSpec::Collect { .. } => "collect",
            GoalSpec::Attack(_) => "attack",
            GoalSpec::Guard(_) => "guard",
            GoalSpec::Craft { .. } => "craft",
            GoalSpec::Give { .. } => "give",
            GoalSpec::Place { .. } => "place",
            GoalSpec::SleepInBed => "sleep_in_bed",
            GoalSpec::EatNow => "eat_now",
        }
    }
}
