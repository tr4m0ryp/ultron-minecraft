//! One smelt step: walk to the furnace, load input and fuel, leave it cooking with the menu
//! closed (the body stays free for its reflexes), come back when the time is up and collect.
//! A stalled furnace with an empty fuel slot is topped up from the inventory.

use super::REACH;
use crate::crafting::{choose_fuel, Recipe, Shape};
use crate::goals::ports::{FurnaceJob, MenuStatus, NavTarget};
use crate::goals::task::{GoalCx, Intent, Plan};
use crate::goals::travel::{Walk, Walker};
use crate::math::{BlockPos, Vec3};

/// Ticks a furnace visit (open, clicks, close) may take.
const VISIT_TIMEOUT: u32 = 200;
/// Extra ticks to wait past the computed cooking time.
const COOK_MARGIN: u64 = 30;
/// Collect visits that found no progress before the furnace counts as stuck.
const MAX_STALLS: u32 = 3;

// One value per tick, never stored; boxing the plan would buy nothing.
#[allow(clippy::large_enum_variant)]
pub enum Smelted {
    Bid(Plan),
    Done,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq)]
enum Phase {
    Walk { load: bool },
    Visit(FurnaceJob),
    Busy { ticks: u32, load: bool },
    Cook { until: u64 },
}

pub struct Smelting {
    pos: BlockPos,
    recipe: Recipe,
    count: u32,
    fuel: Vec<(String, u32)>,
    phase: Phase,
    last_left: Option<u32>,
    stalls: u32,
    /// Collect visits that came back with the output still in the furnace.
    retakes: u32,
    /// A blast furnace or a smoker: half the cooking time.
    fast: bool,
}

impl Smelting {
    pub fn new(pos: BlockPos, recipe: Recipe, count: u32, fuel: Vec<(String, u32)>) -> Self {
        Self {
            pos,
            recipe,
            count,
            fuel,
            phase: Phase::Walk { load: true },
            last_left: None,
            stalls: 0,
            retakes: 0,
            fast: false,
        }
    }

    /// The block at `pos` is a blast furnace or a smoker.
    pub fn fast(mut self) -> Self {
        self.fast = true;
        self
    }

    fn ticks(&self) -> u64 {
        let ticks = match &self.recipe.shape {
            Shape::Smelt { ticks, .. } => u64::from(*ticks),
            _ => 200,
        };
        if self.fast {
            ticks.div_ceil(2)
        } else {
            ticks
        }
    }

    fn at(&self) -> String {
        format!("{} {} {}", self.pos.x, self.pos.y, self.pos.z)
    }

    /// The held input kind with the most items, and the planned fuel still held.
    fn load_job(&self, cx: &GoalCx) -> Result<FurnaceJob, String> {
        let Shape::Smelt { input, .. } = &self.recipe.shape else {
            return Err(format!("{} is not a furnace recipe", self.recipe.id));
        };
        let inv = cx.world.inventory();
        let held = inv
            .iter()
            .filter(|(n, _)| input.accepts(n))
            .max_by_key(|(_, c)| *c);
        let Some((name, have)) = held else {
            return Err(format!("no {} left to smelt", input.0.join("/")));
        };
        let fuel = self
            .fuel
            .iter()
            .map(|(f, n)| (f.clone(), (*n).min(cx.world.count_item(f))))
            .find(|(_, n)| *n > 0);
        Ok(FurnaceJob {
            input: Some((name.clone(), self.count.min(*have))),
            fuel,
        })
    }

    pub fn committed(&mut self) {
        if let Phase::Visit(job) = &self.phase {
            let load = job.input.is_some() || job.fuel.is_some();
            self.phase = Phase::Busy { ticks: 0, load };
        }
    }

    /// Back to the furnace after a suspension; what it holds stays in it.
    pub fn reset(&mut self) {
        if !matches!(self.phase, Phase::Cook { .. }) {
            let load = self.last_left.is_none();
            self.phase = Phase::Walk { load };
        }
    }

    pub fn step(&mut self, cx: &mut GoalCx, walker: &mut Walker) -> Smelted {
        match self.phase.clone() {
            Phase::Walk { load } => {
                let t = NavTarget::Block {
                    pos: self.pos,
                    radius: REACH,
                };
                match walker.walk(cx, t, "goal_smelt_walk") {
                    Walk::Moving(p) => return Smelted::Bid(p),
                    Walk::Failed(f) => {
                        return Smelted::Failed(format!(
                            "cannot reach the furnace at {}: {f:?}",
                            self.at()
                        ))
                    }
                    Walk::Arrived => {}
                }
                let job = match load {
                    true => match self.load_job(cx) {
                        Ok(j) => j,
                        Err(e) => return Smelted::Failed(e),
                    },
                    false => FurnaceJob::default(),
                };
                self.phase = Phase::Visit(job);
                Smelted::Bid(Plan::hold("goal_smelt"))
            }
            Phase::Visit(job) => {
                let face = self.pos.feet_centre() + Vec3::new(0.0, 0.5, 0.0);
                let intent = Intent::Furnace { pos: self.pos, job };
                Smelted::Bid(Plan::face("goal_smelt", face, cx.snap()).intent(intent))
            }
            Phase::Busy { ticks, load } => match cx.world.furnace_status() {
                MenuStatus::Done => self.visited(cx, load),
                MenuStatus::Failed(r) => Smelted::Failed(format!("furnace at {}: {r}", self.at())),
                MenuStatus::Busy | MenuStatus::Idle => {
                    if ticks > VISIT_TIMEOUT {
                        return Smelted::Failed(format!(
                            "the furnace at {} did not open",
                            self.at()
                        ));
                    }
                    if !cx.preempted {
                        self.phase = Phase::Busy {
                            ticks: ticks + 1,
                            load,
                        };
                    }
                    Smelted::Bid(Plan::still("goal_smelt", cx.snap()))
                }
            },
            Phase::Cook { until } => {
                if cx.now() >= until {
                    self.phase = Phase::Walk { load: false };
                }
                Smelted::Bid(Plan::hold("goal_smelt_wait"))
            }
        }
    }

    /// After a visit: done when no input is left, otherwise wait for it to cook, topping
    /// the fuel up when a visit found no progress and an empty fuel slot.
    fn visited(&mut self, cx: &mut GoalCx, load: bool) -> Smelted {
        let seen = cx.world.furnace_seen().unwrap_or_default();
        let left = match (&seen.input, &self.recipe.shape) {
            (Some((n, c)), Shape::Smelt { input, .. }) if input.accepts(n) => *c,
            _ => 0,
        };
        if left == 0 {
            // Live, a visit clicked before the server sent the slots and left the cooked
            // items behind: go back for them rather than call it done.
            let output = seen
                .output
                .as_ref()
                .filter(|(n, _)| *n == self.recipe.result);
            if output.is_some() && self.retakes < MAX_STALLS {
                self.retakes += 1;
                self.phase = Phase::Visit(FurnaceJob::default());
                return Smelted::Bid(Plan::hold("goal_smelt"));
            }
            return Smelted::Done;
        }
        if !load && self.last_left == Some(left) {
            self.stalls += 1;
            if self.stalls > MAX_STALLS {
                return Smelted::Failed(format!("the furnace at {} is not smelting", self.at()));
            }
            if seen.fuel.is_none() {
                let (picks, short) = choose_fuel(&cx.world.inventory(), left);
                let Some(fuel) = picks.into_iter().next() else {
                    return Smelted::Failed(format!(
                        "the furnace at {} ran out of fuel with {left} left; need {} coal",
                        self.at(),
                        (short / 4).div_ceil(8).max(1)
                    ));
                };
                self.phase = Phase::Visit(FurnaceJob {
                    input: None,
                    fuel: Some(fuel),
                });
                return Smelted::Bid(Plan::hold("goal_smelt"));
            }
        }
        self.last_left = Some(left);
        let wait = u64::from(left) * self.ticks() + COOK_MARGIN;
        if load {
            cx.note(format!(
                "smelting {left} at the furnace at {}, ready in {} s",
                self.at(),
                wait / 20
            ));
        }
        self.phase = Phase::Cook {
            until: cx.now() + wait,
        };
        Smelted::Bid(Plan::hold("goal_smelt_wait"))
    }
}
