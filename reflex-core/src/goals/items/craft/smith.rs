//! One smithing step: walk to the table, put the template, the base item and the addition in
//! its three slots and take the result the server shows.

use super::visit::{Visit, Visited};
use crate::crafting::{Ingredient, Recipe, Shape};
use crate::goals::ports::{StationJob, StationMenu};
use crate::goals::task::{GoalCx, Plan};
use crate::goals::travel::Walker;
use crate::math::BlockPos;

/// Smithing menu slots.
const TEMPLATE: usize = 0;
const BASE: usize = 1;
const ADDITION: usize = 2;
const RESULT: usize = 3;

// One value per tick, never stored; boxing the plan would buy nothing.
#[allow(clippy::large_enum_variant)]
pub enum Smithed {
    Bid(Plan),
    Done,
    Failed(String),
}

pub struct Smithing {
    pos: BlockPos,
    recipe: Recipe,
    visit: Option<Visit>,
}

impl Smithing {
    pub fn new(pos: BlockPos, recipe: Recipe) -> Self {
        Self {
            pos,
            recipe,
            visit: None,
        }
    }

    fn job(&self, cx: &GoalCx) -> Result<StationJob, String> {
        let Shape::Smith {
            template,
            base,
            addition,
        } = &self.recipe.shape
        else {
            return Err(format!("{} is not a smithing recipe", self.recipe.id));
        };
        let inv = cx.world.inventory();
        let held = |ing: &Ingredient| {
            inv.iter()
                .filter(|(n, c)| *c > 0 && ing.accepts(n))
                .max_by_key(|(_, c)| *c)
                .map(|(n, _)| n.clone())
                .ok_or(format!("no {} left to smith with", ing.0.join("/")))
        };
        Ok(StationJob {
            menu: StationMenu::Smithing,
            take: vec![TEMPLATE, BASE, ADDITION],
            put: vec![
                (TEMPLATE, held(template)?, 1),
                (BASE, held(base)?, 1),
                (ADDITION, held(addition)?, 1),
            ],
            fuel: None,
            result: vec![RESULT],
        })
    }

    pub fn committed(&mut self) {
        if let Some(v) = &mut self.visit {
            v.committed();
        }
    }

    pub fn reset(&mut self) {
        if let Some(v) = &mut self.visit {
            v.reset();
        }
    }

    pub fn step(&mut self, cx: &mut GoalCx, walker: &mut Walker) -> Smithed {
        if self.visit.is_none() {
            match self.job(cx) {
                Ok(job) => self.visit = Some(Visit::new(self.pos, "smithing table", job)),
                Err(e) => return Smithed::Failed(e),
            }
        }
        let v = self.visit.as_mut().expect("set above");
        match v.step(cx, walker, "goal_smith") {
            Visited::Bid(p) => Smithed::Bid(p),
            Visited::Done => Smithed::Done,
            Visited::Failed(e) => Smithed::Failed(e),
        }
    }
}
