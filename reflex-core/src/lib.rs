//! Reflex layer core: the fast executor that keeps the player alive and carries out goals
//! at 20 ticks per second under a slow LLM brain. No Minecraft library dependency; a motor
//! adapter fills `Snapshot` and applies `MotorOutput`.

pub mod behaviour;
pub mod behaviours;
pub mod channels;
pub mod crafting;
pub mod events;
pub mod executor;
pub mod goals;
pub mod knobs;
pub mod math;
pub mod predict;
pub mod selector;
pub mod snapshot;
pub mod trace;

pub use behaviour::{Behaviour, Bid, Ctx, Priority, Veto};
pub use channels::{Channel, ChannelSet, Commands};
pub use events::{Digest, Event, EventClass, Upward};
pub use executor::Executor;
pub use knobs::{GainName, Knobs, Mode, Response, Scope, Trigger};
pub use math::{Aabb, BlockPos, Vec3};
pub use selector::MotorOutput;
pub use snapshot::Snapshot;
