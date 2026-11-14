use crate::math::{BlockPos, Vec3};
use crate::snapshot::Snapshot;

/// Where a goal wants the legs to go.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NavTarget {
    /// Stand within `radius` blocks (3D) of the block's centre.
    Block { pos: BlockPos, radius: f64 },
    /// Stay within `distance` blocks of a moving entity; the planner re-targets as it moves.
    Entity { id: u32, distance: f64 },
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
}

#[derive(Clone, Debug, PartialEq)]
pub enum NavFailure {
    /// The planner found no path.
    Unreachable,
    /// The entity target is no longer tracked.
    TargetLost,
    /// The path stopped making progress.
    Stalled,
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
}
