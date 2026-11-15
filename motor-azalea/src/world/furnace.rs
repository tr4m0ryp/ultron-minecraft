//! Furnace visits by menu clicks: open the furnace by right-clicking it, take the output,
//! clear a foreign input, load input and fuel, wait for the server's slot corrections, note
//! the three slots and close. The furnace cooks with its menu closed.

use azalea::{
    entity::inventory::Inventory,
    inventory::{operations::QuickMoveClick, Menu},
    Client,
};
use reflex_core::goals::{FurnaceJob, FurnaceSlots, MenuStatus};
use reflex_core::math::BlockPos;

use super::clicks::{self, put};

/// Furnace menu slots (player slots follow from 3).
const INPUT: usize = 0;
const FUEL: usize = 1;
const OUTPUT: usize = 2;
const OPEN_TIMEOUT: u32 = 40;
/// Polls to keep the menu open after the clicks, for the server's corrections to land (see
/// `craft::SETTLE_POLLS`).
const SETTLE_POLLS: u32 = 10;

enum Job {
    Idle,
    Opening {
        job: FurnaceJob,
        polls: u32,
    },
    Settling {
        result: Result<(), String>,
        polls: u32,
    },
    Finished(MenuStatus),
}

pub struct Furnacer {
    job: Job,
    pub seen: Option<FurnaceSlots>,
}

impl Default for Furnacer {
    fn default() -> Self {
        Self {
            job: Job::Idle,
            seen: None,
        }
    }
}

fn furnace_open(bot: &Client) -> bool {
    let ecs = bot.ecs.read();
    ecs.get::<Inventory>(bot.entity)
        .is_some_and(|i| i.id != 0 && matches!(i.container_menu, Some(Menu::Furnace { .. })))
}

fn read(menu: &Menu) -> FurnaceSlots {
    FurnaceSlots {
        input: clicks::slot(menu, INPUT),
        fuel: clicks::slot(menu, FUEL),
        output: clicks::slot(menu, OUTPUT),
    }
}

impl Furnacer {
    pub fn start(&mut self, bot: &Client, pos: BlockPos, job: &FurnaceJob) {
        self.seen = None;
        bot.block_interact(azalea::BlockPos::new(pos.x, pos.y, pos.z));
        self.job = Job::Opening {
            job: job.clone(),
            polls: 0,
        };
    }

    pub fn status(&mut self, bot: &Client) -> MenuStatus {
        match &mut self.job {
            Job::Idle => MenuStatus::Idle,
            Job::Finished(s) => s.clone(),
            Job::Opening { job, polls } => {
                if furnace_open(bot) {
                    let result = run(bot, job);
                    self.job = Job::Settling { result, polls: 0 };
                    return MenuStatus::Busy;
                }
                *polls += 1;
                if *polls > OPEN_TIMEOUT {
                    self.job = Job::Finished(MenuStatus::Failed("the furnace did not open".into()));
                }
                MenuStatus::Busy
            }
            Job::Settling { result, polls } => {
                *polls += 1;
                if *polls < SETTLE_POLLS {
                    return MenuStatus::Busy;
                }
                let handle = bot.get_inventory().ok();
                self.seen = handle.and_then(|h| clicks::menu(&h).ok()).map(|m| read(&m));
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

fn run(bot: &Client, job: &FurnaceJob) -> Result<(), String> {
    let handle = bot.get_inventory().map_err(|e| e.to_string())?;
    let m = clicks::menu(&handle)?;
    if clicks::slot(&m, OUTPUT).is_some() {
        handle.click(QuickMoveClick::Left {
            slot: OUTPUT as u16,
        });
    }
    if let Some((item, n)) = &job.input {
        if clicks::slot(&m, INPUT).is_some_and(|(k, _)| k != *item) {
            handle.click(QuickMoveClick::Left { slot: INPUT as u16 });
        }
        put(&handle, item, *n, INPUT as u16)?;
    }
    if let Some((item, n)) = &job.fuel {
        let m = clicks::menu(&handle)?;
        // A different fuel already in the slot burns first; this one waits for a later visit.
        if clicks::slot(&m, FUEL).is_none_or(|(k, _)| k == *item) {
            put(&handle, item, *n, FUEL as u16)?;
        }
    }
    Ok(())
}
