//! Azalea motor adapter for `reflex-core`: senses the client into snapshots and applies
//! motor output back through the client.

pub mod act;
pub mod io;
pub mod nav;
pub mod ports;
pub mod run;
pub mod sense;
pub mod world;

pub use ports::goal_catalog;
pub use run::{handle, State};
