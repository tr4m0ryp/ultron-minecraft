//! Getting the body somewhere: the shared walker and path follower, and the movement goals.

mod come;
mod follow;
mod go_to;
mod path;
pub mod route;
mod stay;
mod walker;

pub use come::ComeHere;
pub use follow::{person_label, resolve_person, Follow};
pub use go_to::GoTo;
pub use path::{face_point, PathFollower, DIG_LIMIT, STALL_TICKS};
pub use stay::Stay;
pub use walker::{arrived, target_pos, Walk, Walker, MAX_NAV_FAILURES};
