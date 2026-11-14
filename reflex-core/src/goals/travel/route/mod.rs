//! Route planning with every move a player has: walk, step, drop, dig through natural
//! blocks (never straight down, never into water or lava, never inside a protected region
//! or a build), pillar up on placed blocks, bridge a gap, staircase out of a pit. Costs
//! make walking win whenever a walkable route exists.

mod cell;
mod goal;
mod grid;
mod hop;
mod moves;
mod search;
#[cfg(test)]
mod tests;

pub use cell::{drop_of, natural, Cell, Region, Terrain, SCAFFOLD};
pub use grid::Grid;
pub use hop::{Hop, HopKind, Placement};
pub use moves::RouteOpts;
pub use goal::{escaped, open_sky, RouteGoal, SKY_SCAN};
pub use search::{plan, search, Route};
