//! The two furnaces that cook twice as fast: a blast furnace for ores, raw metal and metal
//! gear, a smoker for food. Each of their recipes repeats a furnace recipe (same input, same
//! result, half the time), so the planner keeps planning furnace steps and the craft goal
//! takes a step to the faster block when one stands near.

use super::book::{Ingredient, Recipe, Shape};

pub const BLAST_FURNACE: &str = "blast_furnace";
pub const SMOKER: &str = "smoker";

/// What the faster furnaces take, as (station, result, input).
#[derive(Clone, Debug, Default)]
pub struct Quick(Vec<(&'static str, String, Ingredient)>);

impl Quick {
    pub fn add(&mut self, station: &'static str, result: &str, input: Ingredient) {
        self.0.push((station, result.to_string(), input));
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The faster stations that make the same thing from every input the furnace recipe
    /// `r` takes.
    pub fn stations(&self, r: &Recipe) -> Vec<&'static str> {
        let Shape::Smelt { input, .. } = &r.shape else {
            return Vec::new();
        };
        let takes = |station: &str| {
            input.0.iter().all(|i| {
                self.0
                    .iter()
                    .any(|(s, res, ing)| *s == station && *res == r.result && ing.accepts(i))
            })
        };
        [BLAST_FURNACE, SMOKER]
            .into_iter()
            .filter(|s| takes(s))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn smelt(result: &str, input: &str) -> Recipe {
        Recipe {
            id: result.into(),
            result: result.into(),
            count: 1,
            shape: Shape::Smelt {
                input: Ingredient::one(input),
                ticks: 200,
            },
        }
    }

    #[test]
    fn a_furnace_recipe_names_the_faster_station_that_repeats_it() {
        let mut q = Quick::default();
        q.add(BLAST_FURNACE, "iron_ingot", Ingredient::one("raw_iron"));
        q.add(SMOKER, "cooked_beef", Ingredient::one("beef"));
        assert_eq!(
            q.stations(&smelt("iron_ingot", "raw_iron")),
            [BLAST_FURNACE]
        );
        assert_eq!(q.stations(&smelt("cooked_beef", "beef")), [SMOKER]);
        assert!(q.stations(&smelt("stone", "cobblestone")).is_empty());
        assert!(q.stations(&smelt("iron_ingot", "iron_ore")).is_empty());
    }
}
