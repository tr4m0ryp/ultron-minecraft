use crate::channels::{Commands, InvCmd, MainCmd};
use crate::snapshot::{ItemKind, SelfState, Slot};

/// Eating takes 32 ticks; the lock holds a little longer so the last tick lands.
pub const EATING_TICKS: u32 = 34;
/// Hotbar slot 8 in the body schema (index 7) holds food.
const FOOD_SLOT: Slot = 7;

/// Best food slot. Ranks by saturation when hurt (regeneration needs food >= 18), else by
/// nutrition. Golden apples only when `golden` is set.
pub fn best_food(me: &SelfState, golden: bool) -> Option<Slot> {
    let score = |k: ItemKind| -> Option<f32> {
        match k {
            ItemKind::GoldenApple if golden => Some(100.0),
            ItemKind::Food {
                nutrition,
                saturation,
            } if !golden => Some(if me.health < 20.0 {
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
    let count = |s: &crate::snapshot::ItemStack| {
        if matches!(s.kind, ItemKind::Food { .. }) {
            s.count as u32
        } else {
            0
        }
    };
    me.hotbar.iter().flatten().map(count).sum::<u32>()
        + me.inventory.iter().map(|(_, s)| count(s)).sum::<u32>()
}
