use super::{Channel, ChannelSet};
use crate::math::{BlockPos, Vec3};
use crate::snapshot::Slot;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Speed {
    #[default]
    Stop,
    Sneak,
    Walk,
    Sprint,
}

/// Legs express a world-frame heading; the motor layer turns it into WASD relative to the
/// yaw chosen by whoever owns the gaze.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct LegsCmd {
    /// Desired horizontal direction in world frame (need not be normalised; zero = stand).
    pub heading: Vec3,
    pub speed: Speed,
    pub jump: bool,
}

impl LegsCmd {
    pub fn toward(dir: Vec3, speed: Speed) -> Self {
        Self {
            heading: dir.horizontal(),
            speed,
            jump: false,
        }
    }
    pub fn stop() -> Self {
        Self::default()
    }
    pub fn jumping(mut self) -> Self {
        self.jump = true;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GazeCmd {
    Angles { yaw: f32, pitch: f32 },
    LookAt(Vec3),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MainCmd {
    SelectSlot(Slot),
    /// Start using the held item (eat, drink, place water). Held until StopUse or the lock ends.
    Use,
    StopUse,
    Attack(u32),
    /// Place the held block against the given block, on the face nearest the player.
    PlaceOn(BlockPos),
    /// Place the held block into `cell` by clicking the face of `against` that touches it.
    PlaceAt { cell: BlockPos, against: BlockPos },
    Dig(BlockPos),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OffCmd {
    RaiseShield,
    LowerShield,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvCmd {
    /// Swap the selected hotbar item with the offhand.
    SwapHands,
    /// Move an inventory slot's stack into the offhand.
    ToOffhand(Slot),
    /// Move an inventory slot's stack into a hotbar slot.
    ToHotbar { from: Slot, to: Slot },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatCmd(pub String);

/// One command per channel; a bid claims exactly the channels whose field is `Some`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Commands {
    pub legs: Option<LegsCmd>,
    pub gaze: Option<GazeCmd>,
    pub main: Option<MainCmd>,
    pub off: Option<OffCmd>,
    pub inv: Option<InvCmd>,
    pub chat: Option<ChatCmd>,
}

impl Commands {
    pub fn legs(mut self, c: LegsCmd) -> Self {
        self.legs = Some(c);
        self
    }
    pub fn gaze(mut self, c: GazeCmd) -> Self {
        self.gaze = Some(c);
        self
    }
    pub fn look_at(self, p: Vec3) -> Self {
        self.gaze(GazeCmd::LookAt(p))
    }
    pub fn main(mut self, c: MainCmd) -> Self {
        self.main = Some(c);
        self
    }
    pub fn off(mut self, c: OffCmd) -> Self {
        self.off = Some(c);
        self
    }
    pub fn inv(mut self, c: InvCmd) -> Self {
        self.inv = Some(c);
        self
    }
    pub fn chat(mut self, text: impl Into<String>) -> Self {
        self.chat = Some(ChatCmd(text.into()));
        self
    }

    pub fn channels(&self) -> ChannelSet {
        let mut s = ChannelSet::EMPTY;
        let pairs = [
            (self.legs.is_some(), Channel::Legs),
            (self.gaze.is_some(), Channel::Gaze),
            (self.main.is_some(), Channel::Main),
            (self.off.is_some(), Channel::Off),
            (self.inv.is_some(), Channel::Inv),
            (self.chat.is_some(), Channel::Chat),
        ];
        for (present, c) in pairs {
            if present {
                s = s.with(c);
            }
        }
        s
    }

    /// Copy this command set's entry for one channel into `dst`.
    pub fn copy_channel(&self, c: Channel, dst: &mut Commands) {
        match c {
            Channel::Legs => dst.legs = self.legs,
            Channel::Gaze => dst.gaze = self.gaze,
            Channel::Main => dst.main = self.main,
            Channel::Off => dst.off = self.off,
            Channel::Inv => dst.inv = self.inv,
            Channel::Chat => dst.chat = self.chat.clone(),
        }
    }
}
