use azalea::{inventory::operations::SwapClick, Client};
use reflex_core::channels::InvCmd;

use crate::sense::menu_index;

/// Target slot number the protocol uses for the offhand in a swap click.
const SWAP_OFFHAND: u8 = 40;

/// Ticks after a swap click before the next one is sent. A behaviour repeats its command
/// until the snapshot shows the item where it wants it, and the server's answer takes a
/// tick or two: live, the totem reflex clicked on consecutive ticks, every second click
/// swapped the totem back out, and at the end of 20 staged trials the totem was in the
/// offhand in 15.
const SWAP_SETTLE: u8 = 5;

/// Swap clicks in flight: counts down the ticks until the next one may go out.
#[derive(Default)]
pub struct Swaps {
    wait: u8,
}

/// `selected` is the held hotbar slot. The swap-hands key is a player action whose 26.3
/// ordinal Azalea cannot send (see `protocol`), so it is done as the equivalent inventory
/// swap click of the held slot with the offhand.
pub fn apply(bot: &Client, cmd: Option<InvCmd>, selected: u8, swaps: &mut Swaps) {
    if swaps.wait > 0 {
        swaps.wait -= 1;
        return;
    }
    let Some(cmd) = cmd else { return };
    let sent = match cmd {
        InvCmd::SwapHands => swap(bot, selected, SWAP_OFFHAND),
        InvCmd::ToOffhand(slot) => swap(bot, slot, SWAP_OFFHAND),
        InvCmd::ToHotbar { from, to } if to < 9 => swap(bot, from, to),
        InvCmd::ToHotbar { .. } => false,
    };
    if sent {
        swaps.wait = SWAP_SETTLE;
    }
}

/// Swap click inside the player inventory menu; skipped while another container is open,
/// because slot indices differ there. Returns whether a click went out.
fn swap(bot: &Client, from: u8, target: u8) -> bool {
    let Some(source_slot) = menu_index(from) else {
        return false;
    };
    let Ok(handle) = bot.get_inventory() else {
        return false;
    };
    if handle.id() != 0 {
        return false;
    }
    handle.click(SwapClick {
        source_slot,
        target_slot: target,
    });
    true
}
