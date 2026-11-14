//! Player actions with the 26.3 wire numbering.
//!
//! Minecraft 26.3 inserted CHANGE_DESTROY_DIRECTION at ordinal 1 of
//! `ServerboundPlayerActionPacket.Action` (and appended STAB). Azalea's `Action` enum at the
//! pinned revision keeps the old order, so every variant after START lands one action early
//! on the server: its STOP arrives as ABORT (blocks never break) and RELEASE_USE_ITEM
//! arrives as DROP_ITEM (finishing a meal threw the stack away). The enum is written as its
//! ordinal, so each 26.3 action is sent as the Azalea variant that has the same ordinal.

use azalea::{
    core::{direction::Direction, position::BlockPos},
    protocol::packets::game::s_player_action::{Action, ServerboundPlayerAction},
    Client,
};

/// The 26.3 actions the adapter sends. SWAP_ITEM_WITH_OFFHAND (7) and STAB (8) have no
/// Azalea variant with that ordinal; hand swaps go through an inventory click instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerAction {
    StartDestroy,
    AbortDestroy,
    StopDestroy,
    ReleaseUseItem,
}

fn wire(a: PlayerAction) -> Action {
    match a {
        PlayerAction::StartDestroy => Action::StartDestroyBlock, // 0
        PlayerAction::AbortDestroy => Action::StopDestroyBlock,  // 2
        PlayerAction::StopDestroy => Action::DropAllItems,       // 3
        PlayerAction::ReleaseUseItem => Action::SwapItemWithOffhand, // 6
    }
}

pub fn send(bot: &Client, action: PlayerAction, pos: BlockPos, direction: Direction) {
    bot.write_packet(ServerboundPlayerAction {
        action: wire(action),
        pos,
        direction,
        seq: 0,
    });
}
