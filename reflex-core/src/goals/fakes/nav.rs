use std::cell::RefCell;
use std::rc::Rc;

use crate::goals::ports::{NavFailure, NavStatus, NavTarget, Navigator, Steer};
use crate::goals::travel::route::Cell;
use crate::goals::travel::PathFollower;
use crate::math::BlockPos;
use crate::snapshot::Snapshot;

/// Planner that draws a straight line of nodes at the start height (good enough for the flat
/// test arenas), with configurable unreachable targets and planning delay.
#[derive(Debug, Default)]
pub struct FakeNavState {
    /// Block targets whose block is listed here fail as unreachable.
    pub unreachable: Vec<BlockPos>,
    /// Entity targets with these ids fail as unreachable.
    pub unreachable_entities: Vec<u32>,
    /// Walking-only sky targets fail (the body is shut in).
    pub shut_in: bool,
    /// Ticks spent in `Planning` after each request.
    pub plan_ticks: u32,
    pub requests: u32,
    pub pauses: u32,
    pub cancels: u32,
    pub current: Option<NavTarget>,
    /// The last target requested, kept after the request ends.
    pub last: Option<NavTarget>,
    planning_left: u32,
    follower: PathFollower,
}

#[derive(Clone, Debug, Default)]
pub struct FakeNav(pub Rc<RefCell<FakeNavState>>);

impl FakeNav {
    pub fn state(&self) -> std::cell::RefMut<'_, FakeNavState> {
        self.0.borrow_mut()
    }
}

/// Straight line of feet positions from `a` to `b` at `a`'s height, diagonal steps allowed.
pub fn line(a: BlockPos, b: BlockPos) -> Vec<BlockPos> {
    let mut out = Vec::new();
    let mut p = a;
    while (p.x, p.z) != (b.x, b.z) {
        p = BlockPos::new(p.x + (b.x - p.x).signum(), a.y, p.z + (b.z - p.z).signum());
        out.push(p);
    }
    out
}

/// The standing cell next to `block`, on the side facing `me`, at `me`'s height.
pub fn beside(me: BlockPos, block: BlockPos) -> BlockPos {
    let (dx, dz) = (block.x - me.x, block.z - me.z);
    if dx.abs() >= dz.abs() {
        BlockPos::new(block.x - dx.signum(), me.y, block.z)
    } else {
        BlockPos::new(block.x, me.y, block.z - dz.signum())
    }
}

impl Navigator for FakeNav {
    fn request(&mut self, target: NavTarget, snap: &Snapshot) {
        let mut s = self.0.borrow_mut();
        s.requests += 1;
        s.current = Some(target);
        s.last = Some(target);
        s.planning_left = s.plan_ticks;
        let me = snap.me.pos.block();
        match target {
            NavTarget::Block { pos, .. } => s.follower = PathFollower::new(line(me, pos)),
            NavTarget::Beside { block } => {
                s.follower = PathFollower::new(line(me, beside(me, block)));
            }
            _ => {}
        }
    }

    fn poll(&mut self, snap: &Snapshot) -> NavStatus {
        let mut s = self.0.borrow_mut();
        let Some(target) = s.current else {
            return NavStatus::Idle;
        };
        if s.planning_left > 0 {
            s.planning_left -= 1;
            return NavStatus::Planning;
        }
        match target {
            NavTarget::Sky { dig: false, .. } if s.shut_in => {
                NavStatus::Failed(NavFailure::Unreachable)
            }
            NavTarget::Sky { .. } => NavStatus::Arrived,
            NavTarget::Block { pos, .. } | NavTarget::Beside { block: pos } => {
                if s.unreachable.contains(&pos) {
                    return NavStatus::Failed(NavFailure::Unreachable);
                }
                match s.follower.steer(snap, &|_| Cell::Open) {
                    None => NavStatus::Arrived,
                    Some(Ok(steer)) => NavStatus::Moving(steer),
                    Some(Err(f)) => NavStatus::Failed(f),
                }
            }
            NavTarget::Entity { id, distance } => {
                if s.unreachable_entities.contains(&id) {
                    return NavStatus::Failed(NavFailure::Unreachable);
                }
                let pos = snap.entity(id).map(|e| e.pos).or_else(|| {
                    let c = snap.companion.as_ref()?;
                    (c.entity_id == Some(id)).then_some(c.pos)
                });
                let Some(pos) = pos else {
                    return NavStatus::Failed(NavFailure::TargetLost);
                };
                let d = pos.dist(snap.me.pos);
                if d <= distance {
                    return NavStatus::Arrived;
                }
                NavStatus::Moving(Steer::walk(pos, false, d > 8.0))
            }
        }
    }

    fn pause(&mut self) {
        let mut s = self.0.borrow_mut();
        s.pauses += 1;
        s.current = None;
    }

    fn cancel(&mut self) {
        let mut s = self.0.borrow_mut();
        s.cancels += 1;
        s.current = None;
    }
}
