use crate::channels::{Commands, InvCmd, LegsCmd, MainCmd};
use crate::snapshot::{SelfState, Slot};

/// Commands that bring `slot` into the hand (move it to the hotbar, then select it), or
/// `None` once it is held. The body stands still meanwhile.
pub fn equip(me: &SelfState, slot: Slot) -> Option<Commands> {
    let still = Commands::default().legs(LegsCmd::stop());
    if slot > 8 {
        Some(still.inv(InvCmd::ToHotbar {
            from: slot,
            to: me.selected_slot,
        }))
    } else if slot != me.selected_slot {
        Some(still.main(MainCmd::SelectSlot(slot)))
    } else {
        None
    }
}
