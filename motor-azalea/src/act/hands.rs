use super::{
    dig::DigState,
    protocol::{send, PlayerAction},
};
use azalea::{
    core::entity_id::MinecraftEntityId,
    protocol::packets::game::{s_interact::InteractionHand, s_use_item::ServerboundUseItem},
    Client,
};
use reflex_core::{
    channels::{MainCmd, OffCmd},
    snapshot::{ItemKind, SelfState},
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
    /// Ticks the shield has been up; it blocks from the 5th.
    shield_ticks: u32,
    idle_ticks: u32,
    /// Phase of the 1-in-5 movement duty cycle while an item is in use.
    slow_phase: u8,
    dig: DigState,
    pub(super) menu: super::menu::MenuGuard,
    pub(super) swaps: super::inventory::Swaps,
}

impl HandState {
    /// Efference copy of the off hand for the next snapshot.
    pub fn shield_ticks(&self) -> u32 {
        self.shield_ticks
    }

    /// Vanilla scales movement input by 0.2 and stops sprinting while an item is in use
    /// (eating, shield up); Azalea leaves that out (its `using_item` is hard-coded false),
    /// so the bot ran at full sprint while eating or blocking. The adapter emulates it:
    /// no sprint, and the walk input on one tick in five. `Some(true)` means hold still this
    /// tick, `Some(false)` walk without sprint, `None` not slowed.
    pub fn slowed(&mut self) -> Option<bool> {
        if !(self.shield_up || self.using_main) {
            self.slow_phase = 0;
            return None;
        }
        self.slow_phase = (self.slow_phase + 1) % 5;
        Some(self.slow_phase != 1)
    }

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

/// Right-click with the main-hand item as a use in the air (vanilla `useItem`). Azalea's
/// start-use sends only `UseItemOn` when the crosshair is on a block, and unlike the vanilla
/// client never follows up with `UseItem` when the block ignores it, so a bucket aimed at the
/// ground did nothing. The server ray-casts buckets itself from this packet's rotation.
fn use_in_air(bot: &Client, hand: InteractionHand, look: (f32, f32)) {
    bot.write_packet(ServerboundUseItem {
        hand,
        seq: 0,
        y_rot: look.0,
        x_rot: look.1,
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

/// `look` is the yaw and pitch this tick's output sets, sent with item uses.
pub fn apply_main(
    bot: &Client,
    cmd: Option<MainCmd>,
    st: &mut HandState,
    me: &SelfState,
    look: (f32, f32),
) {
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
            let bucket = me
                .held()
                .is_some_and(|s| matches!(s.kind, ItemKind::WaterBucket | ItemKind::EmptyBucket));
            if bucket {
                // A bucket use is instant; the reflex issuing it owns the one-shot timing.
                use_in_air(bot, InteractionHand::MainHand, look);
            } else if !st.using_main && !st.shield_up {
                use_in_air(bot, InteractionHand::MainHand, look);
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
        // A vanilla client discards attack clicks while an item is in use, so a raised shield
        // must come down (any tick without RaiseShield) before the strike lands next tick.
        MainCmd::Attack(_) if st.shield_up => {}
        MainCmd::Attack(id) => {
            if let Ok(Some(target)) = bot.entity_id_by_minecraft_id(MinecraftEntityId(id as i32)) {
                bot.attack(target);
            }
        }
        // Azalea's entity interact sends the 26.3 packet layout (entity id, hand, location,
        // sneak), twice as the vanilla client does; a sheep sheared by the first ignores the
        // second.
        MainCmd::Interact(id) => {
            if let Ok(Some(target)) = bot.entity_id_by_minecraft_id(MinecraftEntityId(id as i32)) {
                bot.entity_interact(target);
            }
        }
        // The face is the one under the crosshair when the gaze already targets the block,
        // otherwise Azalea falls back to the top face.
        MainCmd::PlaceOn(p) => bot.block_interact(azalea::BlockPos::new(p.x, p.y, p.z)),
        MainCmd::PlaceAt { cell, against } => super::place::place_at(bot, cell, against),
        MainCmd::Dig(_) => {}
    }
}

/// Hold-to-block, as a player holds right click: the shield stays up only while some
/// behaviour commands `RaiseShield` every tick, and comes down on the first tick without it.
/// Before this nothing ever lowered it, so one raise blocked eating and kept the bot at
/// shield walking speed for good.
pub fn apply_off(bot: &Client, cmd: Option<OffCmd>, st: &mut HandState, look: (f32, f32)) {
    match cmd {
        Some(OffCmd::RaiseShield) if !st.shield_up && !st.using_main => {
            use_in_air(bot, InteractionHand::OffHand, look);
            st.shield_up = true;
            st.idle_ticks = 0;
        }
        Some(OffCmd::RaiseShield) => {}
        _ if st.shield_up => {
            release_use(bot);
            st.shield_up = false;
        }
        _ => {}
    }
    st.shield_ticks = if st.shield_up { st.shield_ticks + 1 } else { 0 };
}
