//! One brewing step: walk to the stand, load up to three bottles, the reagent and (when the
//! fuel slot is empty) a blaze powder, leave it brewing with the menu closed (the body stays
//! free for its reflexes), come back after the 20 s it takes and collect the bottles.

use super::visit::{Visit, Visited};
use crate::crafting::{Recipe, Shape, BLAZE_POWDER, BREW_BOTTLES};
use crate::goals::ports::{StationJob, StationMenu};
use crate::goals::task::{GoalCx, Plan};
use crate::goals::travel::Walker;
use crate::math::BlockPos;

/// Brewing stand menu slots.
const BOTTLES: [usize; 3] = [0, 1, 2];
const REAGENT: usize = 3;
const FUEL: usize = 4;
/// Ticks one batch takes on the server.
pub const BREW_TICKS: u64 = 400;
/// Extra ticks to wait past the brewing time.
const BREW_MARGIN: u64 = 40;
/// Loads tried before the stand counts as not brewing.
const MAX_LOADS: u32 = 2;

// One value per tick, never stored; boxing the plan would buy nothing.
#[allow(clippy::large_enum_variant)]
pub enum Brewed {
    Bid(Plan),
    Done,
    Failed(String),
}

enum Phase {
    Start,
    Load(Visit),
    Brew { until: u64 },
    Collect(Visit),
}

pub struct Brewing {
    pos: BlockPos,
    recipe: Recipe,
    bottles: u32,
    phase: Phase,
    /// Result bottles held before the first load, and bottles loaded.
    held_before: u32,
    loads: u32,
}

impl Brewing {
    pub fn new(pos: BlockPos, recipe: Recipe, bottles: u32) -> Self {
        Self {
            pos,
            recipe,
            bottles: bottles.clamp(1, BREW_BOTTLES),
            phase: Phase::Start,
            held_before: 0,
            loads: 0,
        }
    }

    fn at(&self) -> String {
        format!("{} {} {}", self.pos.x, self.pos.y, self.pos.z)
    }

    /// Empties the stand, then puts in the bottles, one reagent and the fuel offer.
    fn load_job(&mut self, cx: &GoalCx) -> Result<StationJob, String> {
        let Shape::Brew { input, reagent } = &self.recipe.shape else {
            return Err(format!("{} is not a brewing recipe", self.recipe.id));
        };
        let inv = cx.world.inventory();
        let held = |ing: &crate::crafting::Ingredient| {
            inv.iter()
                .filter(|(n, c)| *c > 0 && ing.accepts(n))
                .max_by_key(|(_, c)| *c)
                .cloned()
        };
        let (Some((bottle, have)), Some((with, _))) = (held(input), held(reagent)) else {
            return Err(format!(
                "no {} or no {} left to brew {}",
                input.0.join("/"),
                reagent.0.join("/"),
                self.recipe.result
            ));
        };
        self.bottles = self.bottles.min(have);
        let mut put: Vec<(usize, String, u32)> = (0..self.bottles as usize)
            .map(|i| (BOTTLES[i], bottle.clone(), 1))
            .collect();
        put.push((REAGENT, with, 1));
        Ok(StationJob {
            menu: StationMenu::BrewingStand,
            take: vec![BOTTLES[0], BOTTLES[1], BOTTLES[2], REAGENT],
            put,
            fuel: Some((FUEL, BLAZE_POWDER.to_string())),
            result: Vec::new(),
        })
    }

    pub fn committed(&mut self) {
        if let Phase::Load(v) | Phase::Collect(v) = &mut self.phase {
            v.committed();
        }
    }

    /// Back to the stand after a suspension; what it holds stays in it and brews on.
    pub fn reset(&mut self) {
        if let Phase::Load(v) | Phase::Collect(v) = &mut self.phase {
            v.reset();
        }
    }

    fn collect(&self) -> Visit {
        let job = StationJob {
            menu: StationMenu::BrewingStand,
            // A reagent still in its slot means the batch never ran: it comes back too.
            take: vec![BOTTLES[0], BOTTLES[1], BOTTLES[2], REAGENT],
            put: Vec::new(),
            fuel: None,
            result: Vec::new(),
        };
        Visit::new(self.pos, "brewing stand", job)
    }

    pub fn step(&mut self, cx: &mut GoalCx, walker: &mut Walker) -> Brewed {
        let hold = |note| Brewed::Bid(Plan::hold(note));
        match &mut self.phase {
            Phase::Start => {
                if self.loads == 0 {
                    self.held_before = cx.world.count_item(&self.recipe.result);
                }
                if self.loads >= MAX_LOADS {
                    return Brewed::Failed(format!(
                        "the brewing stand at {} is not brewing; it needs 1 {BLAZE_POWDER}",
                        self.at()
                    ));
                }
                self.loads += 1;
                match self.load_job(cx) {
                    Ok(job) => self.phase = Phase::Load(Visit::new(self.pos, "brewing stand", job)),
                    Err(e) => return Brewed::Failed(e),
                }
                hold("goal_brew")
            }
            Phase::Load(v) => match v.step(cx, walker, "goal_brew") {
                Visited::Bid(p) => Brewed::Bid(p),
                Visited::Failed(e) => Brewed::Failed(e),
                Visited::Done => {
                    let wait = BREW_TICKS + BREW_MARGIN;
                    cx.note(format!(
                        "brewing {} {} at the stand at {}, ready in {} s",
                        self.bottles,
                        self.recipe.result,
                        self.at(),
                        wait / 20
                    ));
                    self.phase = Phase::Brew {
                        until: cx.now() + wait,
                    };
                    hold("goal_brew_wait")
                }
            },
            Phase::Brew { until } => {
                if cx.now() >= *until {
                    self.phase = Phase::Collect(self.collect());
                }
                hold("goal_brew_wait")
            }
            Phase::Collect(v) => match v.step(cx, walker, "goal_brew") {
                Visited::Bid(p) => Brewed::Bid(p),
                Visited::Failed(e) => Brewed::Failed(e),
                Visited::Done => {
                    let got = cx.world.count_item(&self.recipe.result);
                    if got >= self.held_before + self.bottles {
                        return Brewed::Done;
                    }
                    // The bottles came back as they went in: the stand had no fuel.
                    self.phase = Phase::Start;
                    hold("goal_brew")
                }
            },
        }
    }
}
