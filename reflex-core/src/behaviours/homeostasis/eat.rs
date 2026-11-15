use crate::channels::{Commands, InvCmd, MainCmd};
use crate::snapshot::{ItemKind, SelfState, Slot};

/// Eating takes 32 ticks; the lock holds a little longer so the last tick lands.
pub const EATING_TICKS: u32 = 34;

/// A meal may start only if it finishes before any archer that sees us can land an arrow:
/// out of sight, or in the reload after its last shot. Eating slows the player to a crawl
/// and keeps the shield down, so a meal in an archer's line of fire is 32 ticks as a target.
pub fn meal_is_safe(snap: &crate::snapshot::Snapshot) -> bool {
    crate::behaviours::defence::safe_for(snap, EATING_TICKS as f64)
}
/// Hotbar slot 8 in the body schema (index 7) holds food.
const FOOD_SLOT: Slot = 7;

/// Best food slot. Ranks by saturation when hurt (regeneration needs food >= 18), else by
/// nutrition. Golden apples only when `golden` is set. Raw food counts: an explicit meal
/// or an emergency eats what there is.
pub fn best_food(me: &SelfState, golden: bool) -> Option<Slot> {
    pick_food(me, golden, true)
}

/// Best food that is ready to eat: raw meat, fish and potatoes stay for the furnace (a
/// cooked steak feeds 8, a raw one 3), as a player keeps them while merely peckish.
pub fn best_ready_food(me: &SelfState) -> Option<Slot> {
    pick_food(me, false, false)
}

fn pick_food(me: &SelfState, golden: bool, raw: bool) -> Option<Slot> {
    let score = |k: ItemKind| -> Option<f32> {
        match k {
            ItemKind::GoldenApple if golden => Some(100.0),
            ItemKind::Food {
                nutrition,
                saturation,
                cookable,
            } if !golden && (raw || !cookable) => Some(if me.health < 20.0 {
                saturation
            } else {
                nutrition as f32
            }),
            _ => None,
        }
    };
    let hotbar = me
        .hotbar
        .iter()
        .enumerate()
        .filter_map(|(i, s)| Some((i as Slot, score(s.as_ref()?.kind)?)));
    let inv = me
        .inventory
        .iter()
        .filter_map(|(i, s)| Some((*i, score(s.kind)?)));
    hotbar
        .chain(inv)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// Next step toward eating from `slot`: bring it to the hotbar, select it, then use.
pub fn eat_commands(me: &SelfState, slot: Slot) -> Commands {
    if slot > 8 {
        Commands::default().inv(InvCmd::ToHotbar {
            from: slot,
            to: FOOD_SLOT,
        })
    } else if me.selected_slot != slot {
        Commands::default().main(MainCmd::SelectSlot(slot))
    } else {
        Commands::default().main(MainCmd::Use)
    }
}

/// Total servings of ordinary food carried.
pub fn food_servings(me: &SelfState) -> u32 {
    servings(me, |_| true)
}

/// Servings of raw food that a furnace would make a better meal of.
pub fn raw_servings(me: &SelfState) -> u32 {
    servings(me, |cookable| cookable)
}

fn servings(me: &SelfState, keep: impl Fn(bool) -> bool) -> u32 {
    let count = |s: &crate::snapshot::ItemStack| match s.kind {
        ItemKind::Food { cookable, .. } if keep(cookable) => s.count as u32,
        _ => 0,
    };
    me.hotbar.iter().flatten().map(count).sum::<u32>()
        + me.inventory.iter().map(|(_, s)| count(s)).sum::<u32>()
}
