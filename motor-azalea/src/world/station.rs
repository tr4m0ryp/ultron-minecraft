//! Brewing stand and smithing table visits by menu clicks, in the furnace's manner: open the
//! block by right-clicking it, wait for the server's slots, empty the slots the job names,
//! move the job's items in, wait for the server's answer, take the result when the job
//! expects one (a smithing table shows it only after the server has seen all three items),
//! and close. A stand brews with its menu closed.

use azalea::{
    entity::inventory::Inventory,
    inventory::{operations::QuickMoveClick, Menu},
    Client,
};
use reflex_core::goals::{MenuStatus, StationJob, StationMenu};
use reflex_core::math::BlockPos;

use super::clicks::{self, put, Handle};

const OPEN_TIMEOUT: u32 = 40;
/// Polls the menu stays open before the first click (see `furnace::OPEN_SETTLE`).
const OPEN_SETTLE: u32 = 3;
/// Polls after the clicks, for the server's slot updates to land.
const SETTLE_POLLS: u32 = 10;
/// Polls a result may take to show after the items went in.
const RESULT_POLLS: u32 = 40;

enum Job {
    Idle,
    Opening {
        job: StationJob,
        polls: u32,
        open_for: u32,
    },
    /// The items are in; the result (if the job expects one) is still to be taken.
    Loaded {
        job: StationJob,
        polls: u32,
    },
    Closing {
        result: Result<(), String>,
        polls: u32,
    },
    Finished(MenuStatus),
}

pub struct Visitor {
    job: Job,
}

impl Default for Visitor {
    fn default() -> Self {
        Self { job: Job::Idle }
    }
}

fn name(menu: StationMenu) -> &'static str {
    match menu {
        StationMenu::BrewingStand => "brewing stand",
        StationMenu::Smithing => "smithing table",
    }
}

fn is_open(bot: &Client, want: StationMenu) -> bool {
    let ecs = bot.ecs.read();
    ecs.get::<Inventory>(bot.entity).is_some_and(|i| {
        i.id != 0
            && matches!(
                (&i.container_menu, want),
                (Some(Menu::BrewingStand { .. }), StationMenu::BrewingStand)
                    | (Some(Menu::Smithing { .. }), StationMenu::Smithing)
            )
    })
}

fn filled(handle: &Handle, slot: usize) -> bool {
    clicks::menu(handle).is_ok_and(|m| clicks::slot(&m, slot).is_some())
}

fn take(handle: &Handle, slot: usize) {
    if filled(handle, slot) {
        handle.click(QuickMoveClick::Left { slot: slot as u16 });
    }
}

/// Empties the job's `take` slots, moves its items in and offers the fuel.
fn load(bot: &Client, job: &StationJob) -> Result<(), String> {
    let handle = bot.get_inventory().map_err(|e| e.to_string())?;
    for &slot in &job.take {
        take(&handle, slot);
    }
    for (slot, item, n) in &job.put {
        put(&handle, item, *n, *slot as u16)?;
    }
    if let Some((slot, item)) = &job.fuel {
        let held = clicks::held(&clicks::menu(&handle)?);
        if !filled(&handle, *slot) && held.iter().any(|(k, c)| k == item && *c > 0) {
            put(&handle, item, 1, *slot as u16)?;
        }
    }
    Ok(())
}

impl Visitor {
    pub fn start(&mut self, bot: &Client, pos: BlockPos, job: &StationJob) {
        bot.block_interact(azalea::BlockPos::new(pos.x, pos.y, pos.z));
        self.job = Job::Opening {
            job: job.clone(),
            polls: 0,
            open_for: 0,
        };
    }

    pub fn status(&mut self, bot: &Client) -> MenuStatus {
        match &mut self.job {
            Job::Idle => MenuStatus::Idle,
            Job::Finished(s) => s.clone(),
            Job::Opening {
                job,
                polls,
                open_for,
            } => {
                if is_open(bot, job.menu) {
                    *open_for += 1;
                    if *open_for < OPEN_SETTLE {
                        return MenuStatus::Busy;
                    }
                    self.job = match load(bot, job) {
                        Ok(()) => Job::Loaded {
                            job: job.clone(),
                            polls: 0,
                        },
                        Err(e) => Job::Closing {
                            result: Err(e),
                            polls: 0,
                        },
                    };
                    return MenuStatus::Busy;
                }
                *polls += 1;
                if *polls > OPEN_TIMEOUT {
                    let why = format!("the {} did not open", name(job.menu));
                    self.job = Job::Finished(MenuStatus::Failed(why));
                }
                MenuStatus::Busy
            }
            Job::Loaded { job, polls } => {
                *polls += 1;
                if *polls < SETTLE_POLLS {
                    return MenuStatus::Busy;
                }
                let handle = bot.get_inventory().ok();
                let shown = |h: &Handle| job.result.iter().all(|&s| filled(h, s));
                let result = match handle.as_ref() {
                    None => Err("menu closed".to_string()),
                    Some(h) if shown(h) => {
                        for &slot in &job.result {
                            take(h, slot);
                        }
                        Ok(())
                    }
                    Some(_) if *polls < SETTLE_POLLS + RESULT_POLLS => return MenuStatus::Busy,
                    Some(_) => Err(format!("the {} showed no result", name(job.menu))),
                };
                self.job = Job::Closing { result, polls: 0 };
                MenuStatus::Busy
            }
            Job::Closing { result, polls } => {
                *polls += 1;
                if *polls < SETTLE_POLLS {
                    return MenuStatus::Busy;
                }
                // Closing hands whatever is left in a smithing table back to the player.
                clicks::close(bot);
                self.job = Job::Finished(match result.clone() {
                    Ok(()) => MenuStatus::Done,
                    Err(e) => MenuStatus::Failed(e),
                });
                MenuStatus::Busy
            }
        }
    }
}
