//! Crafting and smelting along a planned dependency chain. Every round plans afresh from
//! the inventory and runs the plan's first step: a 2x2 craft in the inventory grid, a 3x3
//! craft at a crafting table (walked to when one stands near, else a held one is put down
//! beside the player), a furnace visit, a batch at a brewing stand or one item at a smithing
//! table. Missing raw materials end the goal once, with the planner's shopping list.

mod brew;
mod furnace;
mod replan;
mod smith;
mod station;
mod visit;

use crate::crafting::Recipe;
use crate::goals::ports::{MenuStatus, NavTarget, Navigator};
use crate::goals::task::{GoalCx, GoalTask, Intent, Plan, Step};
use crate::goals::travel::{Walk, Walker};
use crate::math::{BlockPos, Vec3};
use brew::{Brewed, Brewing};
use furnace::{Smelted, Smelting};
use smith::{Smithed, Smithing};
use station::{Placed, Placing};

pub use station::STATION_RADIUS as TABLE_RADIUS;
pub const CRAFT_TIMEOUT_TICKS: u32 = 200;
const REACH: f64 = 3.5;
/// Steps one goal may run (the iron pickaxe chain from logs takes seven).
const MAX_STEPS: u32 = 48;
/// Adapter failures tolerated before the goal gives up.
const MAX_FAILS: u32 = 2;
/// Ticks between a finished step and the next plan.
const SETTLE_TICKS: u32 = 6;

// One phase per goal; boxing the sub-machines would buy nothing.
#[allow(clippy::large_enum_variant)]
enum Phase {
    /// Let the server's inventory updates land before planning again.
    Settle(u32),
    Plan,
    Place(Placing),
    Walk(BlockPos, Recipe, u32),
    Craft(Recipe, u32, Option<BlockPos>),
    Waiting(u32),
    Smelt(Smelting),
    Brew(Brewing),
    Smith(Smithing),
}

pub struct Craft {
    item: String,
    count: u32,
    smelt_only: bool,
    baseline: Option<u32>,
    phase: Phase,
    walker: Walker,
    steps: u32,
    fails: u32,
    placed: Vec<String>,
    /// How the last step was made ("craft", "smelt", "brew", "smith"), for the outcome.
    made_by: &'static str,
    /// A brewing stand this goal found without fuel: the next plan asks for blaze powder.
    stand_empty: bool,
}

impl Craft {
    pub fn new(item: String, count: u32) -> Self {
        Self {
            item,
            count: count.max(1),
            smelt_only: false,
            baseline: None,
            phase: Phase::Plan,
            walker: Walker::default(),
            steps: 0,
            fails: 0,
            placed: Vec::new(),
            made_by: "craft",
            stand_empty: false,
        }
    }

    /// The `smelt` goal: the same machine, with the last step bound to a furnace.
    pub fn smelt(item: String, count: u32) -> Self {
        Self {
            smelt_only: true,
            ..Self::new(item, count)
        }
    }

    fn done_text(&self, got: u32) -> String {
        let verb = match (self.smelt_only, self.made_by) {
            (true, _) => "smelted",
            (_, "brew") => "brewed",
            (_, "smith") => "smithed",
            _ => "crafted",
        };
        let mut s = format!("{verb} {got} {}", self.item);
        if !self.placed.is_empty() {
            s.push_str(&format!("; put down {}", self.placed.join(", ")));
        }
        s
    }

    fn adapter_failed(&mut self, what: String) -> Step {
        self.fails += 1;
        if self.fails > MAX_FAILS {
            return Step::Failed(what);
        }
        self.phase = Phase::Plan;
        Step::Bid(Plan::hold("goal_craft"))
    }
}

impl GoalTask for Craft {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        let have = cx.world.count_item(&self.item);
        let base = *self.baseline.get_or_insert(have);
        let got = have.saturating_sub(base);
        if got >= self.count && !matches!(self.phase, Phase::Place(_)) {
            return Step::Done(self.done_text(got));
        }
        let hold = || Step::Bid(Plan::hold("goal_craft"));
        match &mut self.phase {
            Phase::Settle(0) => {
                self.phase = Phase::Plan;
                hold()
            }
            Phase::Settle(t) => {
                *t -= 1;
                hold()
            }
            Phase::Plan => match self.replan(cx, self.count - got) {
                Ok(()) => hold(),
                Err(e) => Step::Failed(e),
            },
            Phase::Place(placing) => match placing.step(cx) {
                Placed::Bid(p) => Step::Bid(p),
                Placed::Failed(e) => Step::Failed(e),
                Placed::Done(p) => {
                    let what = format!("a {} at {} {} {}", placing.block, p.x, p.y, p.z);
                    cx.note(format!("put down {what}"));
                    self.placed.push(what);
                    self.phase = Phase::Plan;
                    hold()
                }
            },
            Phase::Walk(t, recipe, crafts) => {
                let target = NavTarget::Block {
                    pos: *t,
                    radius: REACH,
                };
                match self.walker.walk(cx, target, "goal_craft_walk") {
                    Walk::Arrived => {
                        self.phase = Phase::Craft(recipe.clone(), *crafts, Some(*t));
                        hold()
                    }
                    Walk::Moving(p) => Step::Bid(p),
                    Walk::Failed(f) => {
                        Step::Failed(format!("cannot reach the crafting table: {f:?}"))
                    }
                }
            }
            Phase::Craft(recipe, crafts, table) => {
                let intent = Intent::Craft {
                    recipe: recipe.clone(),
                    crafts: *crafts,
                    table: *table,
                };
                let plan = match table {
                    Some(t) => Plan::face(
                        "goal_craft",
                        t.feet_centre() + Vec3::new(0.0, 0.5, 0.0),
                        cx.snap(),
                    ),
                    None => Plan::still("goal_craft", cx.snap()),
                };
                Step::Bid(plan.intent(intent))
            }
            Phase::Waiting(ticks) => match cx.world.craft_status() {
                MenuStatus::Done => {
                    self.phase = Phase::Settle(SETTLE_TICKS);
                    hold()
                }
                MenuStatus::Failed(r) => self.adapter_failed(format!("crafting failed: {r}")),
                MenuStatus::Busy | MenuStatus::Idle => {
                    if *ticks > CRAFT_TIMEOUT_TICKS {
                        return self.adapter_failed("crafting timed out".into());
                    }
                    if !cx.preempted {
                        *ticks += 1;
                    }
                    Step::Bid(Plan::still("goal_craft", cx.snap()))
                }
            },
            Phase::Smelt(s) => match s.step(cx, &mut self.walker) {
                Smelted::Bid(p) => Step::Bid(p),
                Smelted::Done => {
                    self.phase = Phase::Settle(SETTLE_TICKS);
                    hold()
                }
                Smelted::Failed(e) => Step::Failed(e),
            },
            Phase::Brew(b) => match b.step(cx, &mut self.walker) {
                Brewed::Bid(p) => Step::Bid(p),
                Brewed::Done => {
                    self.phase = Phase::Settle(SETTLE_TICKS);
                    hold()
                }
                Brewed::Failed(e) => {
                    // One plan more, this time asking for the fuel the stand lacked.
                    if e.contains("is not brewing") && !self.stand_empty {
                        self.stand_empty = true;
                        self.phase = Phase::Plan;
                        return hold();
                    }
                    Step::Failed(e)
                }
            },
            Phase::Smith(m) => match m.step(cx, &mut self.walker) {
                Smithed::Bid(p) => Step::Bid(p),
                Smithed::Done => {
                    self.phase = Phase::Settle(SETTLE_TICKS);
                    hold()
                }
                Smithed::Failed(e) => self.adapter_failed(e),
            },
        }
    }

    fn committed(&mut self) {
        match &mut self.phase {
            Phase::Craft(..) => self.phase = Phase::Waiting(0),
            Phase::Smelt(s) => s.committed(),
            Phase::Brew(b) => b.committed(),
            Phase::Smith(m) => m.committed(),
            _ => {}
        }
    }

    fn stop(&mut self, nav: &mut dyn Navigator) {
        self.walker.reset();
        match &mut self.phase {
            // Whatever the furnace holds stays there: resume the smelt, do not re-plan.
            Phase::Smelt(s) => s.reset(),
            // A stand keeps brewing with nobody at it: come back to it.
            Phase::Brew(b) => b.reset(),
            Phase::Place(p) => {
                p.stop(nav);
                self.phase = Phase::Plan;
            }
            _ => self.phase = Phase::Plan,
        }
    }
}
