//! Using the inventory: crafting, giving, placing.

mod craft;
mod equip;
mod give;
mod place;
mod surfaces;

pub use craft::{Craft, CRAFT_TIMEOUT_TICKS, TABLE_RADIUS};
pub use equip::equip;
pub use give::Give;
pub use place::Place;
pub use surfaces::{opens_on_click, replaceable};
