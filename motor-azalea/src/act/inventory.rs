use azalea::{
    inventory::operations::SwapClick,
    protocol::packets::game::s_player_action::{Action, ServerboundPlayerAction},
    Client,
};
use reflex_core::channels::InvCmd;

use crate::sense::menu_index;

/// Target slot number the protocol uses for the offhand in a swap click.
const SWAP_OFFHAND: u8 = 40;

pub fn apply(bot: &Client, cmd: Option<InvCmd>) {
    let Some(cmd) = cmd else { return };
    match cmd {
        InvCmd::SwapHands => bot.write_packet(ServerboundPlayerAction {
            action: Action::SwapItemWithOffhand,
            pos: Default::default(),
            direction: Default::default(),
            seq: 0,
        }),
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
