//! In-memory implementations of the goal ports for tests and offline runs. Each fake keeps
//! its state behind a shared handle so a test (or a simulator) can inspect and change it
//! while the goal runner owns the boxed port.

mod nav;
pub mod recipes;
mod world;

pub use nav::{FakeNav, FakeNavState};
pub use world::{FakeWorld, FakeWorldState};
