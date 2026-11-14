use std::cell::RefCell;
use std::rc::Rc;

use crate::goals::ports::{NavFailure, NavStatus, NavTarget, Navigator, Steer};
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
    /// Ticks spent in `Planning` after each request.
    pub plan_ticks: u32,
    pub requests: u32,
    pub pauses: u32,
    pub cancels: u32,
    pub current: Option<NavTarget>,
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

impl Navigator for FakeNav {
    fn request(&mut self, target: NavTarget, snap: &Snapshot) {
        let mut s = self.0.borrow_mut();
        s.requests += 1;
        s.current = Some(target);
        s.planning_left = s.plan_ticks;
        if let NavTarget::Block { pos, .. } = target {
            s.follower = PathFollower::new(line(snap.me.pos.block(), pos));
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
            NavTarget::Block { pos, .. } => {
                if s.unreachable.contains(&pos) {
                    return NavStatus::Failed(NavFailure::Unreachable);
                }
                match s.follower.steer(snap) {
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
                NavStatus::Moving(Steer {
                    waypoint: pos,
                    jump: false,
                    sprint: d > 8.0,
                })
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
