//! Using the inventory: crafting, giving, placing.

mod break_at;
mod craft;
mod dig_out;
mod equip;
mod fill;
mod give;
pub(crate) mod place;
pub(crate) mod surfaces;

pub use break_at::BreakAt;
pub use craft::{Craft, CRAFT_TIMEOUT_TICKS, TABLE_RADIUS};
pub use dig_out::{dig_out, DIG_OUT_LIMIT};
pub use equip::equip;
pub use fill::{Fill, FILL_MAX};
pub use give::Give;
pub use place::Place;
pub use surfaces::{opens_on_click, replaceable, soft_natural};
