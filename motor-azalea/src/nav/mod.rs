//! `reflex_core::goals::Navigator` on Azalea's A* planner.
//!
//! UNTESTED LIVE: compiles against the pinned Azalea revision but has never driven a bot on
//! a server. Steering reuses `reflex_core::goals::travel::PathFollower`, which is tested.
//!
//! Yielding the legs: this navigator never moves the body itself. The goal turns each
//! `Steer` into a legs bid, so a reflex that outbids it simply wins the legs; the goal then
//! calls `pause` (the path is dropped) and re-requests once it owns the legs again, which
//! re-plans from wherever the reflex left the body.

mod plan;

use azalea::Client;
use reflex_core::{
    goals::{travel::PathFollower, NavFailure, NavStatus, NavTarget, Navigator},
    math::Vec3,
    snapshot::Snapshot,
};

pub use plan::set_max_drop;
use plan::{Planner, Poll};

/// Re-plan toward a moving entity once it is this far from where the path was aimed.
const RETARGET: f64 = 3.0;

pub struct AzaleaNav {
    bot: Client,
    target: Option<NavTarget>,
    planner: Planner,
    follower: Option<PathFollower>,
    partial: bool,
    aimed_at: Option<Vec3>,
}

impl AzaleaNav {
    pub fn new(bot: Client) -> Self {
        Self {
            bot,
            target: None,
            planner: Planner::default(),
            follower: None,
            partial: false,
            aimed_at: None,
        }
    }

    fn goal_of(t: NavTarget, snap: &Snapshot) -> Option<(Vec3, f64)> {
        match t {
            NavTarget::Block { pos, radius } => {
                Some((pos.feet_centre() + Vec3::new(0.0, 0.5, 0.0), radius))
            }
            NavTarget::Entity { id, distance } => {
                let pos = snap.entity(id).map(|e| e.pos).or_else(|| {
                    let c = snap.companion.as_ref()?;
                    (c.entity_id == Some(id)).then_some(c.pos)
                })?;
                Some((pos, distance))
            }
        }
    }

    fn plan(&mut self, t: NavTarget, snap: &Snapshot) -> NavStatus {
        let Some((goal, radius)) = Self::goal_of(t, snap) else {
            return NavStatus::Failed(NavFailure::TargetLost);
        };
        self.aimed_at = Some(goal);
        if self.planner.start(&self.bot, snap.me.pos, goal, radius) {
            NavStatus::Planning
        } else {
            NavStatus::Failed(NavFailure::Unreachable)
        }
    }
}

impl Navigator for AzaleaNav {
    fn request(&mut self, target: NavTarget, snap: &Snapshot) {
        self.target = Some(target);
        self.follower = None;
        self.plan(target, snap);
    }

    fn poll(&mut self, snap: &Snapshot) -> NavStatus {
        let Some(t) = self.target else {
            return NavStatus::Idle;
        };
        let mut busy = false;
        match self.planner.poll() {
            Poll::Busy => {
                busy = true;
                if self.follower.is_none() {
                    return NavStatus::Planning;
                }
            }
            Poll::Lost => return self.plan(t, snap),
            Poll::Ready(p) => {
                if p.nodes.is_empty() {
                    return if p.partial {
                        NavStatus::Failed(NavFailure::Unreachable)
                    } else {
                        NavStatus::Arrived
                    };
                }
                self.partial = p.partial;
                self.follower = Some(PathFollower::new(p.nodes));
            }
            Poll::Idle => {}
        }
        if let NavTarget::Entity { .. } = t {
            let Some((now, _)) = Self::goal_of(t, snap) else {
                return NavStatus::Failed(NavFailure::TargetLost);
            };
            let moved = self.aimed_at.is_none_or(|a| a.dist(now) > RETARGET);
            if moved && !busy {
                self.plan(t, snap);
            }
        }
        let Some(f) = self.follower.as_mut() else {
            return self.plan(t, snap);
        };
        match f.steer(snap) {
            Some(Ok(s)) => NavStatus::Moving(s),
            Some(Err(e)) => {
                self.follower = None;
                NavStatus::Failed(e)
            }
            None if self.partial => {
                self.follower = None;
                self.plan(t, snap)
            }
            None => {
                self.follower = None;
                NavStatus::Arrived
            }
        }
    }

    fn pause(&mut self) {
        self.cancel();
    }

    fn cancel(&mut self) {
        self.planner.cancel();
        self.target = None;
        self.follower = None;
        self.aimed_at = None;
    }
}
