//! Crafting by inventory clicks: the 2x2 grid of the player inventory, or a crafting table
//! opened by right-clicking it. Verified live on 26.3: planks and sticks in the 2x2 grid, a
//! wooden pickaxe at a table.

use azalea::{
    entity::inventory::Inventory,
    inventory::{
        operations::{PickupClick, QuickMoveClick},
        Menu,
    },
    Client,
};
use reflex_core::{goals::MenuStatus, math::BlockPos};

use super::names::item_matches;
use super::recipes::{lookup, Shaped};

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
        recipe: Shaped,
        crafts: u32,
        polls: u32,
    },
    Settling {
        result: Result<(), String>,
        polls: u32,
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
    pub fn start(&mut self, bot: &Client, item: &str, crafts: u32, table: Option<BlockPos>) {
        let Some(recipe) = lookup(item) else {
            self.job = Job::Finished(MenuStatus::Failed(format!("no recipe for {item}")));
            return;
        };
        self.job = match table {
            None => outcome(run(bot, &recipe, crafts, 2)),
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
                    self.job = Job::Settling { result, polls: 0 };
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
            Job::Settling { result, polls } => {
                *polls += 1;
                if *polls < SETTLE_POLLS {
                    return MenuStatus::Busy;
                }
                if let Ok(h) = bot.get_inventory() {
                    h.close();
                }
                self.job = outcome(result.clone());
                MenuStatus::Busy
            }
        }
    }
}

/// Lays the recipe out `crafts` deep in a `grid`-wide crafting grid (result slot 0, grid
/// from slot 1 row-major), shift-clicks the result, and returns any leftovers.
fn run(bot: &Client, r: &Shaped, crafts: u32, grid: usize) -> Result<(), String> {
    if r.width() > grid || r.height() > grid {
        return Err("needs a crafting table".into());
    }
    let handle = bot.get_inventory().map_err(|e| e.to_string())?;
    if grid == 2 && handle.id() != 0 {
        return Err("another container is open".into());
    }
    for (row, line) in r.rows.iter().enumerate() {
        for (col, c) in line.chars().enumerate() {
            let Some(ing) = r.ingredient(c) else {
                continue;
            };
            let slot = (1 + row * grid + col) as u16;
            let mut placed = 0;
            while placed < crafts {
                let menu = handle
                    .menu()
                    .map_err(|e| e.to_string())?
                    .ok_or("menu closed")?;
                let src = menu
                    .player_slots_range()
                    .find(|&i| menu.slot(i).is_some_and(|s| item_matches(ing, s)))
                    .ok_or(format!("ran out of {ing}"))?;
                let have = menu.slot(src).map_or(0, |s| s.count().max(0) as u32);
                let put = have.min(crafts - placed);
                handle.click(PickupClick::Left {
                    slot: Some(src as u16),
                });
                for _ in 0..put {
                    handle.click(PickupClick::Right { slot: Some(slot) });
                }
                if have > put {
                    handle.click(PickupClick::Left {
                        slot: Some(src as u16),
                    });
                }
                placed += put;
            }
        }
    }
    handle.click(QuickMoveClick::Left { slot: 0 });
    for g in 1..=(grid * grid) as u16 {
        let left = handle
            .menu()
            .ok()
            .flatten()
            .and_then(|m| m.slot(g as usize).map(|s| !s.is_empty()))
            .unwrap_or(false);
        if left {
            handle.click(QuickMoveClick::Left { slot: g });
        }
    }
    Ok(())
}
