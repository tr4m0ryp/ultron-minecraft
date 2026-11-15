//! The contract every goal implements: a resumable state machine stepped once per tick.

use super::ledger::Ledger;
use super::ports::{FurnaceJob, Navigator, World};
use super::runner::Reporter;
use crate::behaviour::Ctx;
use crate::channels::{Commands, LegsCmd, MainCmd};
use crate::crafting::Recipe;
use crate::knobs::Knobs;
use crate::math::{BlockPos, Vec3};
use crate::snapshot::Snapshot;

/// A side effect the adapter performs outside the channel vocabulary. It is committed only
/// on a tick the selector granted the goal its channels, so reflexes pre-empt it too.
#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    Craft {
        recipe: Recipe,
        crafts: u32,
        table: Option<BlockPos>,
    },
    /// One furnace visit (collect, load, close).
    Furnace {
        pos: BlockPos,
        job: FurnaceJob,
    },
    Toss {
        item: String,
        count: u32,
    },
}

/// What a goal wants from the body this tick.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub commands: Commands,
    pub degraded: Option<Commands>,
    pub note: &'static str,
    pub lock: u32,
    pub intent: Option<Intent>,
}

impl Plan {
    pub fn new(note: &'static str, commands: Commands) -> Self {
        Self {
            commands,
            degraded: None,
            note,
            lock: 0,
            intent: None,
        }
    }
    /// Stand still: claims the legs so the goal keeps its place in the selector.
    pub fn hold(note: &'static str) -> Self {
        Self::new(note, Commands::default().legs(LegsCmd::stop()))
    }
    /// Stand still facing `p`, with the main hand claimed but idle, ready for a menu action.
    pub fn face(note: &'static str, p: Vec3, snap: &Snapshot) -> Self {
        Self::new(
            note,
            Commands::default()
                .legs(LegsCmd::stop())
                .look_at(p)
                .main(MainCmd::SelectSlot(snap.me.selected_slot)),
        )
    }
    /// Stand still with the main hand claimed but idle, for a menu action.
    pub fn still(note: &'static str, snap: &Snapshot) -> Self {
        Self::new(
            note,
            Commands::default()
                .legs(LegsCmd::stop())
                .main(MainCmd::SelectSlot(snap.me.selected_slot)),
        )
    }
    pub fn degraded(mut self, c: Commands) -> Self {
        self.degraded = Some(c);
        self
    }
    pub fn lock(mut self, ticks: u32) -> Self {
        self.lock = ticks;
        self
    }
    pub fn intent(mut self, i: Intent) -> Self {
        self.intent = Some(i);
        self
    }
}

// One value per goal per tick; boxing the plan would buy nothing.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    Bid(Plan),
    /// Nothing to do this tick; the goal stays active.
    Idle,
    Done(String),
    Failed(String),
}

/// Everything a goal may read or drive during one step.
pub struct GoalCx<'a> {
    pub ctx: &'a Ctx<'a>,
    pub nav: &'a mut dyn Navigator,
    pub world: &'a mut dyn World,
    pub rep: &'a mut Reporter,
    /// Blocks the player placed: never dug by a route, never cleared by a placement.
    pub ledger: &'a Ledger,
    /// The goal bid last tick and lost: a reflex holds the body. Timers should not run.
    pub preempted: bool,
}

impl GoalCx<'_> {
    pub fn snap(&self) -> &Snapshot {
        self.ctx.snap
    }
    pub fn knobs(&self) -> &Knobs {
        self.ctx.knobs
    }
    pub fn now(&self) -> u64 {
        self.ctx.snap.tick
    }
    pub fn me_pos(&self) -> Vec3 {
        self.ctx.snap.me.pos
    }
    /// Report progress at coarse milestones (quarters of `total`).
    pub fn progress(&mut self, done: u32, total: u32, text: impl Into<String>) {
        self.rep.progress(self.ctx, done, total, text.into());
    }
    /// Report a notable change for an open-ended goal, rate limited.
    pub fn note(&mut self, text: impl Into<String>) {
        self.rep.note(self.ctx, text.into());
    }
    /// The goal cannot advance right now; reported once per reason until it clears.
    pub fn blocked(&mut self, reason: impl Into<String>) {
        self.rep.blocked(self.ctx, reason.into());
    }
}

pub trait GoalTask {
    fn step(&mut self, cx: &mut GoalCx) -> Step;
    /// The runner executed this tick's intent.
    fn committed(&mut self) {}
    /// The goal was suspended (replaced) or ended; release anything held.
    fn stop(&mut self, _nav: &mut dyn Navigator) {}
}
