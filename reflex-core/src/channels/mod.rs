//! Body channels and the commands a behaviour may issue on each. Every channel has at most
//! one owner per tick; the selector decides who.

mod commands;

pub use commands::{ChatCmd, Commands, GazeCmd, InvCmd, LegsCmd, MainCmd, OffCmd, Speed};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Channel {
    Legs,
    Gaze,
    Main,
    Off,
    Inv,
    Chat,
}

impl Channel {
    pub const ALL: [Channel; 6] = [
        Channel::Legs,
        Channel::Gaze,
        Channel::Main,
        Channel::Off,
        Channel::Inv,
        Channel::Chat,
    ];

    fn bit(self) -> u8 {
        1 << (self as u8)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChannelSet(u8);

impl ChannelSet {
    pub const EMPTY: ChannelSet = ChannelSet(0);

    pub fn with(mut self, c: Channel) -> Self {
        self.0 |= c.bit();
        self
    }
    pub fn contains(self, c: Channel) -> bool {
        self.0 & c.bit() != 0
    }
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
    pub fn iter(self) -> impl Iterator<Item = Channel> {
        Channel::ALL.into_iter().filter(move |c| self.contains(*c))
    }
}

impl FromIterator<Channel> for ChannelSet {
    fn from_iter<I: IntoIterator<Item = Channel>>(it: I) -> Self {
        it.into_iter().fold(ChannelSet::EMPTY, ChannelSet::with)
    }
}
