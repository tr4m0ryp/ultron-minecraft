//! `MotorOutput` -> Azalea client calls.

mod hands;
mod inventory;
mod movement;

use azalea::Client;
use reflex_core::{MotorOutput, snapshot::SelfState};

pub use hands::HandState;

/// Apply one tick of motor output. `me` is the snapshot the output was computed from.
pub fn apply(bot: &Client, out: &MotorOutput, me: &SelfState, hands: &mut HandState) {
    movement::apply(bot, out);
    hands.sync(me.using_item);
    hands::apply_main(bot, out.main, hands, me);
    hands::apply_off(bot, out.off, hands);
    inventory::apply(bot, out.inv);
    if let Some(text) = &out.chat {
        bot.chat(text.as_str());
    }
}
