//! Name-based reads of the player inventory menu, and tossing.

use azalea::{entity::inventory::Inventory, inventory::operations::ThrowClick, Client};
use reflex_core::snapshot::Slot;

use super::names::item_matches;

/// Player menu indices of the main inventory and hotbar (9..=35 main, 36..=44 hotbar).
pub const PLAYER_SLOTS: std::ops::RangeInclusive<usize> = 9..=44;
const HOTBAR_START: usize = 36;

/// Player menu index to the reflex slot numbering (0..=8 hotbar, 9..=35 main).
pub fn reflex_slot(menu_index: usize) -> Option<Slot> {
    match menu_index {
        9..=35 => Some(menu_index as Slot),
        36..=44 => Some((menu_index - HOTBAR_START) as Slot),
        _ => None,
    }
}

fn with_menu<T>(bot: &Client, f: impl FnOnce(&azalea::inventory::Menu) -> T) -> Option<T> {
    let ecs = bot.ecs.read();
    ecs.get::<Inventory>(bot.entity)
        .map(|i| f(&i.inventory_menu))
}

pub fn count(bot: &Client, kind: &str) -> u32 {
    with_menu(bot, |m| {
        PLAYER_SLOTS
            .filter_map(|i| m.slot(i))
            .filter(|s| item_matches(kind, s))
            .map(|s| s.count().max(0) as u32)
            .sum()
    })
    .unwrap_or(0)
}

pub fn total(bot: &Client) -> u32 {
    with_menu(bot, |m| {
        PLAYER_SLOTS
            .filter_map(|i| m.slot(i))
            .map(|s| s.count().max(0) as u32)
            .sum()
    })
    .unwrap_or(0)
}

pub fn slot_of(bot: &Client, kind: &str) -> Option<Slot> {
    with_menu(bot, |m| {
        (HOTBAR_START..=44)
            .chain(9..HOTBAR_START)
            .find(|&i| m.slot(i).is_some_and(|s| item_matches(kind, s)))
            .and_then(reflex_slot)
    })
    .flatten()
}

pub fn free(bot: &Client) -> u32 {
    with_menu(bot, |m| {
        PLAYER_SLOTS
            .filter(|&i| m.slot(i).is_none_or(|s| s.is_empty()))
            .count() as u32
    })
    .unwrap_or(0)
}

/// Throws up to `n` items of `kind` with drop-key clicks in the player inventory menu.
/// Skipped while another container is open (its slot numbering differs).
pub fn toss(bot: &Client, kind: &str, n: u32) -> u32 {
    let Some(stacks) = with_menu(bot, |m| {
        PLAYER_SLOTS
            .filter_map(|i| Some((i, m.slot(i)?.clone())))
            .filter(|(_, s)| item_matches(kind, s))
            .map(|(i, s)| (i as u16, s.count().max(0) as u32))
            .collect::<Vec<_>>()
    }) else {
        return 0;
    };
    let Ok(handle) = bot.get_inventory() else {
        return 0;
    };
    if handle.id() != 0 {
        return 0;
    }
    let mut left = n;
    for (slot, have) in stacks {
        if left == 0 {
            break;
        }
        if have <= left {
            handle.click(ThrowClick::All { slot });
            left -= have;
        } else {
            for _ in 0..left {
                handle.click(ThrowClick::Single { slot });
            }
            left = 0;
        }
    }
    n - left
}
