//! Crafting by inventory clicks: the 2x2 grid of the player inventory, or a crafting table
//! opened by right-clicking it. The planner's recipe carries tag ingredients; `layout`
//! picks one concrete held kind per cell, and the laid grid is checked against the recipe
//! (shaped patterns also match mirrored) before the result is taken.

use azalea::{
    entity::inventory::Inventory,
    inventory::{operations::QuickMoveClick, Menu},
    Client,
};
use reflex_core::crafting::{layout, matches, Recipe};
use reflex_core::{goals::MenuStatus, math::BlockPos};

use super::clicks::{self, put};

/// Polls (ticks) to wait for the crafting table menu to open.
const OPEN_TIMEOUT: u32 = 40;
/// Polls to keep the table open after the clicks. The server's slot corrections (the
/// result it computed, the consumed grid) arrive addressed to the table's menu; Azalea
/// copies that menu's player slots into the inventory on close, so closing in the same
/// tick as the clicks kept the client's wrong prediction (live: pickaxe crafted on the
/// server, the bot saw 1 plank and 1 stick and failed the goal).
const SETTLE_POLLS: u32 = 10;

enum Job {
    Idle,
    Opening {
        recipe: Recipe,
        crafts: u32,
        polls: u32,
    },
    Settling {
        result: Result<(), String>,
        polls: u32,
        table: bool,
    },
    Finished(MenuStatus),
}

pub struct Crafter {
    job: Job,
}

impl Default for Crafter {
    fn default() -> Self {
        Self { job: Job::Idle }
    }
}

fn outcome(r: Result<(), String>) -> Job {
    Job::Finished(match r {
        Ok(()) => MenuStatus::Done,
        Err(e) => MenuStatus::Failed(e),
    })
}

fn table_open(bot: &Client) -> bool {
    let ecs = bot.ecs.read();
    ecs.get::<Inventory>(bot.entity)
        .is_some_and(|i| i.id != 0 && matches!(i.container_menu, Some(Menu::Crafting { .. })))
}

impl Crafter {
    pub fn start(&mut self, bot: &Client, recipe: &Recipe, crafts: u32, table: Option<BlockPos>) {
        let recipe = recipe.clone();
        self.job = match table {
            // The 2x2 grid settles too: the result's slot update arrives a few ticks later,
            // and a goal that re-plans before it lands plans from a stale inventory (live: 4
            // torches crafted, then "cannot craft torch" from the coal already spent).
            None => Job::Settling {
                result: run(bot, &recipe, crafts, 2),
                polls: 0,
                table: false,
            },
            Some(p) => {
                bot.block_interact(azalea::BlockPos::new(p.x, p.y, p.z));
                Job::Opening {
                    recipe,
                    crafts,
                    polls: 0,
                }
            }
        };
    }

    pub fn status(&mut self, bot: &Client) -> MenuStatus {
        match &mut self.job {
            Job::Idle => MenuStatus::Idle,
            Job::Finished(s) => s.clone(),
            Job::Opening {
                recipe,
                crafts,
                polls,
            } => {
                if table_open(bot) {
                    let result = run(bot, recipe, *crafts, 3);
                    self.job = Job::Settling {
                        result,
                        polls: 0,
                        table: true,
                    };
                    return MenuStatus::Busy;
                }
                *polls += 1;
                if *polls > OPEN_TIMEOUT {
                    self.job =
                        Job::Finished(MenuStatus::Failed("the crafting table did not open".into()));
                    return self.status(bot);
                }
                MenuStatus::Busy
            }
            Job::Settling {
                result,
                polls,
                table,
            } => {
                *polls += 1;
                if *polls < SETTLE_POLLS {
                    return MenuStatus::Busy;
                }
                if *table {
                    clicks::close(bot);
                }
                self.job = outcome(result.clone());
                MenuStatus::Busy
            }
        }
    }
}

/// Lays the recipe out in a `grid`-wide crafting grid (result slot 0, grid from slot 1
/// row-major) as deep as the held kinds allow, shift-clicks the result, and returns any
/// leftovers.
fn run(bot: &Client, r: &Recipe, crafts: u32, grid: usize) -> Result<(), String> {
    if r.needs_table() && grid < 3 {
        return Err("needs a crafting table".into());
    }
    let handle = bot.get_inventory().map_err(|e| e.to_string())?;
    if grid == 2 && handle.id() != 0 {
        return Err("another container is open".into());
    }
    let held = clicks::held(&clicks::menu(&handle)?);
    let lay = layout(r, grid, &held, crafts).ok_or(format!("not enough held for {}", r.result))?;
    for (cell, item) in &lay.cells {
        put(&handle, item, lay.crafts, 1 + *cell as u16)?;
    }
    let m = clicks::menu(&handle)?;
    let laid: Vec<Option<String>> = (1..=grid * grid)
        .map(|i| clicks::slot(&m, i).map(|(n, _)| n))
        .collect();
    let fits = matches(r, grid, &laid);
    if fits {
        handle.click(QuickMoveClick::Left { slot: 0 });
    }
    for g in 1..=(grid * grid) as u16 {
        let left = clicks::menu(&handle).is_ok_and(|m| clicks::slot(&m, g as usize).is_some());
        if left {
            handle.click(QuickMoveClick::Left { slot: g });
        }
    }
    match fits {
        true => Ok(()),
        false => Err(format!("the laid grid did not match the {} recipe", r.id)),
    }
}
