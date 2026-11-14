use azalea::{inventory::operations::SwapClick, Client};
use reflex_core::channels::InvCmd;

use crate::sense::menu_index;

/// Target slot number the protocol uses for the offhand in a swap click.
const SWAP_OFFHAND: u8 = 40;

/// `selected` is the held hotbar slot. The swap-hands key is a player action whose 26.3
/// ordinal Azalea cannot send (see `protocol`), so it is done as the equivalent inventory
/// swap click of the held slot with the offhand.
pub fn apply(bot: &Client, cmd: Option<InvCmd>, selected: u8) {
    let Some(cmd) = cmd else { return };
    match cmd {
        InvCmd::SwapHands => swap(bot, selected, SWAP_OFFHAND),
        InvCmd::ToOffhand(slot) => swap(bot, slot, SWAP_OFFHAND),
        InvCmd::ToHotbar { from, to } if to < 9 => swap(bot, from, to),
        InvCmd::ToHotbar { .. } => {}
    }
}

/// Swap click inside the player inventory menu; skipped while another container is open,
/// because slot indices differ there.
fn swap(bot: &Client, from: u8, target: u8) {
    let Some(source_slot) = menu_index(from) else {
        return;
    };
    let Ok(handle) = bot.get_inventory() else {
        return;
    };
    if handle.id() != 0 {
        return;
    }
    handle.click(SwapClick {
        source_slot,
        target_slot: target,
    });
}
