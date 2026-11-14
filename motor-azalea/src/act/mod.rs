//! `MotorOutput` -> Azalea client calls.

mod dig;
mod hands;
mod inventory;
mod menu;
mod movement;
mod protocol;

use azalea::Client;
use reflex_core::{snapshot::SelfState, MotorOutput};

pub use hands::HandState;

/// Apply one tick of motor output. `me` is the snapshot the output was computed from.
pub fn apply(bot: &Client, out: &MotorOutput, me: &SelfState, hands: &mut HandState) {
    movement::apply(bot, out, hands.slowed());
    hands.sync(me.using_item);
    hands::apply_main(bot, out.main, hands, me, (out.yaw, out.pitch));
    hands::apply_off(bot, out.off, hands, (out.yaw, out.pitch));
    inventory::apply(bot, out.inv, me.selected_slot);
    menu::apply(bot, &mut hands.menu);
    if let Some(text) = &out.chat {
        bot.chat(text.as_str());
    }
}
