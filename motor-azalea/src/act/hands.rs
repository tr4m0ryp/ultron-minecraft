use super::{
    dig::DigState,
    protocol::{send, PlayerAction},
};
use azalea::{
    core::entity_id::MinecraftEntityId, interact::StartUseItemEvent,
    protocol::packets::game::s_interact::InteractionHand, Client,
};
use reflex_core::{
    channels::{MainCmd, OffCmd},
    snapshot::SelfState,
};

/// Ticks without the server confirming an item use before the adapter assumes the use ended
/// (finished eating, shield knocked down) and allows a new one.
const USE_GRACE_TICKS: u32 = 6;

/// Item-use bookkeeping. Azalea only offers a one-shot right click, so the adapter tracks
/// whether a use is in progress and releases it on request.
#[derive(Default)]
pub struct HandState {
    using_main: bool,
    shield_up: bool,
    idle_ticks: u32,
    dig: DigState,
}

impl HandState {
    /// Reconcile with the server-reported using-item flag from the latest snapshot.
    pub fn sync(&mut self, server_using: bool) {
        if server_using {
            self.idle_ticks = 0;
        } else if self.using_main || self.shield_up {
            self.idle_ticks += 1;
            if self.idle_ticks >= USE_GRACE_TICKS {
                self.using_main = false;
                self.shield_up = false;
                self.idle_ticks = 0;
            }
        }
    }
}

fn start_use(bot: &Client, hand: InteractionHand) {
    bot.ecs.write().write_message(StartUseItemEvent {
        entity: bot.entity,
        hand,
        force_block: None,
    });
}

fn release_use(bot: &Client) {
    send(
        bot,
        PlayerAction::ReleaseUseItem,
        Default::default(),
        Default::default(),
    );
}

pub fn apply_main(bot: &Client, cmd: Option<MainCmd>, st: &mut HandState, me: &SelfState) {
    let dig = match cmd {
        Some(MainCmd::Dig(p)) => Some(p),
        _ => None,
    };
    st.dig.tick(bot, dig);
    let Some(cmd) = cmd else { return };
    match cmd {
        MainCmd::SelectSlot(slot) => {
            if slot < 9 && slot != me.selected_slot {
                bot.set_selected_hotbar_slot(slot);
            }
        }
        MainCmd::Use => {
            if !st.using_main && !st.shield_up {
                start_use(bot, InteractionHand::MainHand);
                st.using_main = true;
                st.idle_ticks = 0;
            }
        }
        MainCmd::StopUse => {
            if st.using_main {
                release_use(bot);
                st.using_main = false;
            }
        }
        MainCmd::Attack(id) => {
            if let Ok(Some(target)) = bot.entity_id_by_minecraft_id(MinecraftEntityId(id as i32)) {
                bot.attack(target);
            }
        }
        // The face is the one under the crosshair when the gaze already targets the block,
        // otherwise Azalea falls back to the top face.
        MainCmd::PlaceOn(p) => bot.block_interact(azalea::BlockPos::new(p.x, p.y, p.z)),
        MainCmd::Dig(_) => {}
    }
}

pub fn apply_off(bot: &Client, cmd: Option<OffCmd>, st: &mut HandState) {
    match cmd {
        Some(OffCmd::RaiseShield) if !st.shield_up && !st.using_main => {
            start_use(bot, InteractionHand::OffHand);
            st.shield_up = true;
            st.idle_ticks = 0;
        }
        Some(OffCmd::LowerShield) if st.shield_up => {
            release_use(bot);
            st.shield_up = false;
        }
        _ => {}
    }
}
