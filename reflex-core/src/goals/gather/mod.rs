//! Getting resources: mining blocks and picking up drops.

mod collect;
mod mine;

pub use collect::Collect;
pub use mine::{Mine, DIG_LIMIT_TICKS, SEARCH_RADIUS};
