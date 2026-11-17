//! The two library-agnostic ports goals depend on: a path planner and a world model with
//! recipes and menu actions. The motor adapter implements both; `goals::fakes` holds
//! in-memory versions for tests.

mod nav;
mod world;

pub use nav::{NavFailure, NavStatus, NavTarget, Navigator, Steer, Work};
pub use world::{
    kind_matches, name_matches, FurnaceJob, FurnaceSlots, ItemDrop, MenuStatus, StationJob,
    StationMenu, ToolNeed, World,
};
