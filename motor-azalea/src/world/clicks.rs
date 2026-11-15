//! Inventory clicks shared by the crafting grid and the furnace: moving an exact number of
//! one item kind from the player's slots into a menu slot.

use azalea::inventory::{operations::PickupClick, Menu};
use azalea::Client;

use super::names::item_name;

pub type Handle = azalea::container::ContainerHandleRef;

pub fn menu(handle: &Handle) -> Result<Menu, String> {
    handle
        .menu()
        .map_err(|e| e.to_string())?
        .ok_or("menu closed".into())
}

/// Item names and counts in the open menu's player slots.
pub fn held(menu: &Menu) -> Vec<(String, u32)> {
    let mut out: Vec<(String, u32)> = Vec::new();
    for s in menu.player_slots_range().filter_map(|i| menu.slot(i)) {
        let Some(n) = item_name(s) else {
            continue;
        };
        let c = s.count().max(0) as u32;
        match out.iter_mut().find(|(k, _)| k == n) {
            Some((_, k)) => *k += c,
            None => out.push((n.to_string(), c)),
        }
    }
    out
}

/// The name and count in menu slot `i`, `None` when empty.
pub fn slot(menu: &Menu, i: usize) -> Option<(String, u32)> {
    let s = menu.slot(i)?;
    item_name(s).map(|n| (n.to_string(), s.count().max(0) as u32))
}

/// Right-clicks `n` items of exactly `item` into `target`, one source stack at a time.
pub fn put(handle: &Handle, item: &str, n: u32, target: u16) -> Result<(), String> {
    let mut placed = 0;
    while placed < n {
        let m = menu(handle)?;
        let src = m
            .player_slots_range()
            .find(|&i| m.slot(i).is_some_and(|s| item_name(s) == Some(item)))
            .ok_or(format!("ran out of {item}"))?;
        let have = m.slot(src).map_or(0, |s| s.count().max(0) as u32);
        let take = have.min(n - placed);
        handle.click(PickupClick::Left {
            slot: Some(src as u16),
        });
        for _ in 0..take {
            handle.click(PickupClick::Right { slot: Some(target) });
        }
        if have > take {
            handle.click(PickupClick::Left {
                slot: Some(src as u16),
            });
        }
        placed += take;
    }
    Ok(())
}

pub fn close(bot: &Client) {
    if let Ok(h) = bot.get_inventory() {
        h.close();
    }
}
