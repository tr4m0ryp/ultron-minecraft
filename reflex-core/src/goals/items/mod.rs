//! Using the inventory: crafting, giving, placing.

mod craft;
mod equip;
mod fill;
mod give;
mod place;
mod surfaces;

pub use craft::{Craft, CRAFT_TIMEOUT_TICKS, TABLE_RADIUS};
pub use equip::equip;
pub use fill::{Fill, FILL_MAX};
pub use give::Give;
pub use place::Place;
pub use surfaces::{opens_on_click, replaceable};
