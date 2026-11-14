use crate::goals::travel::route::{Placement, Region};
use crate::math::{BlockPos, Vec3};
use crate::snapshot::Snapshot;

/// Where a goal wants the legs to go.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NavTarget {
    /// Stand within `radius` blocks (3D) of the block's centre.
    Block { pos: BlockPos, radius: f64 },
    /// Stay within `distance` blocks of a moving entity; the planner re-targets as it moves.
    Entity { id: u32, distance: f64 },
    /// Stand level with `block` in a neighbouring column, to mine it from the side.
    Beside { block: BlockPos },
    /// Get out under open sky (escaping an enclosure); `hint` is the last place known to
    /// have it. Without `dig` only walking routes count (a probe for "enclosed").
    Sky { hint: BlockPos, dig: bool },
}

/// Hand work a path needs before the legs can go on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Work {
    /// Break this block (the way is blocked).
    Dig(BlockPos),
    /// Place a scaffold block; `click` is false on the ticks between attempts, when the
    /// body only aims.
    Place { at: Placement, click: bool },
}

/// One tick of steering: the next waypoint along the planned path. The goal turns it into a
/// legs bid, so reflexes pre-empt path following through the ordinary selector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Steer {
    pub waypoint: Vec3,
    /// The next path edge climbs a block.
    pub jump: bool,
    /// The remaining straight run is long enough to sprint.
    pub sprint: bool,
    /// Dig or place first; the legs hold the waypoint meanwhile.
    pub work: Option<Work>,
    /// Approach the waypoint sneaking (to the edge of a block, never off it).
    pub sneak: bool,
}

impl Steer {
    pub fn walk(waypoint: Vec3, jump: bool, sprint: bool) -> Self {
        Self {
            waypoint,
            jump,
            sprint,
            work: None,
            sneak: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum NavFailure {
    /// The planner found no path.
    Unreachable,
    /// The entity target is no longer tracked.
    TargetLost,
    /// The path stopped making progress.
    Stalled,
    /// A placement the path needs has no scaffold block in hand.
    NoScaffold,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NavStatus {
    /// No request, or paused or cancelled.
    Idle,
    /// A request is being planned; hold still.
    Planning,
    Moving(Steer),
    Arrived,
    Failed(NavFailure),
}

/// Path planner port. Requests are fire-and-forget; `poll` is called once per tick by the
/// goal that owns the request and must stay cheap (planning runs elsewhere).
pub trait Navigator {
    /// Plan toward `target`, replacing any previous request.
    fn request(&mut self, target: NavTarget, snap: &Snapshot);
    /// Advance along the current path given the latest snapshot.
    fn poll(&mut self, snap: &Snapshot) -> NavStatus;
    /// A reflex took the legs: stop following the path but keep nothing that assumes the
    /// body is still where it was. The goal re-requests (re-plans) when it gets the legs back.
    fn pause(&mut self);
    /// Drop the request entirely.
    fn cancel(&mut self);
    /// Regions the path must never break (the thought layer's protected builds).
    fn protect(&mut self, _regions: &[Region]) {}
    /// Scaffold blocks the path placed since the last call, for a goal to clean up.
    fn take_placed(&mut self) -> Vec<BlockPos> {
        Vec::new()
    }
}
