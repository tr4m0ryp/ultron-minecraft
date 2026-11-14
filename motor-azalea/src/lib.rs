//! Azalea motor adapter for `reflex-core`: senses the client into snapshots and applies
//! motor output back through the client.

pub mod act;
pub mod io;
pub mod run;
pub mod sense;

pub use run::{handle, State};
