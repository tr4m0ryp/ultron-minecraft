//! `reflex_core::goals::Navigator` on reflex-core's route planner (walk, dig, pillar,
//! bridge) over the client's world.
//!
//! Yielding the legs: this navigator never moves the body itself. The goal turns each
//! `Steer` into a bid, so a reflex that outbids it simply wins the body; the goal then
//! calls `pause` (the path is dropped) and re-requests once it owns the body again, which
//! re-plans from wherever the reflex left it.

mod plan;
mod terrain;

use azalea::Client;
use reflex_core::{
    goals::{
        travel::{
            route::{Region, RouteGoal, RouteOpts},
            PathFollower,
        },
        Keep, NavFailure, NavStatus, NavTarget, Navigator,
    },
    math::{BlockPos, Vec3},
    snapshot::Snapshot,
};

pub use plan::set_max_drop;
use plan::{max_drop, Planner, Poll};
pub use terrain::LiveTerrain;

/// Re-plan toward a moving entity once it is this far from where the path was aimed.
const RETARGET: f64 = 3.0;

pub struct AzaleaNav {
    bot: Client,
    target: Option<NavTarget>,
    planner: Planner,
    follower: Option<PathFollower>,
    partial: bool,
    aimed_at: Option<Vec3>,
    protected: Vec<Region>,
    kept: Keep,
    placed: Vec<BlockPos>,
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
            protected: Vec::new(),
            kept: Keep::default(),
            placed: Vec::new(),
        }
    }

    fn entity_pos(id: u32, snap: &Snapshot) -> Option<Vec3> {
        snap.entity(id).map(|e| e.pos).or_else(|| {
            let c = snap.companion.as_ref()?;
            (c.entity_id == Some(id)).then_some(c.pos)
        })
    }

    fn goal_of(t: NavTarget, snap: &Snapshot) -> Option<(RouteGoal, Vec3)> {
        let mid = Vec3::new(0.0, 0.5, 0.0);
        Some(match t {
            NavTarget::Block { pos, radius } => {
                let centre = pos.feet_centre() + mid;
                (RouteGoal::Near { centre, radius }, centre)
            }
            NavTarget::Entity { id, distance } => {
                let p = Self::entity_pos(id, snap)?;
                let centre = p + mid;
                let goal = RouteGoal::Near {
                    centre,
                    radius: distance,
                };
                (goal, p)
            }
            NavTarget::Beside { block } => (RouteGoal::Beside { block }, block.feet_centre()),
            NavTarget::Sky { hint, .. } => (RouteGoal::Sky { hint }, hint.feet_centre()),
        })
    }

    fn opts(&self, t: NavTarget) -> RouteOpts {
        if let NavTarget::Sky { dig: false, .. } = t {
            return RouteOpts::walk_only(max_drop());
        }
        RouteOpts {
            max_drop: max_drop(),
            scaffold: crate::world::scaffold_count(&self.bot),
            protected: self.protected.clone(),
            kept: self.kept.clone(),
            ..RouteOpts::default()
        }
    }

    fn plan(&mut self, t: NavTarget, snap: &Snapshot) -> NavStatus {
        let Some((goal, aim)) = Self::goal_of(t, snap) else {
            return NavStatus::Failed(NavFailure::TargetLost);
        };
        self.aimed_at = Some(aim);
        let from = (snap.me.pos + Vec3::new(0.0, 0.01, 0.0)).block();
        let opts = self.opts(t);
        if self.planner.start(&self.bot, from, goal, opts) {
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
                // A walking probe for the sky that only gets partway means "shut in".
                let probe = matches!(t, NavTarget::Sky { dig: false, .. });
                if p.hops.is_empty() || (probe && p.partial) {
                    return if p.partial {
                        NavStatus::Failed(NavFailure::Unreachable)
                    } else {
                        NavStatus::Arrived
                    };
                }
                self.partial = p.partial;
                self.follower = Some(PathFollower::route(p.hops));
            }
            Poll::Idle => {}
        }
        if let NavTarget::Entity { id, .. } = t {
            let Some(now) = Self::entity_pos(id, snap) else {
                return NavStatus::Failed(NavFailure::TargetLost);
            };
            let moved = self.aimed_at.is_none_or(|a| a.dist(now) > RETARGET);
            let working = self.follower.as_ref().is_some_and(|f| f.working());
            if moved && !busy && !working {
                self.plan(t, snap);
            }
        }
        let Some(f) = self.follower.as_mut() else {
            return self.plan(t, snap);
        };
        let terrain = LiveTerrain::new(&self.bot);
        let steer = f.steer(snap, &terrain);
        self.placed.extend(f.take_placed());
        match steer {
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

    fn protect(&mut self, regions: &[Region]) {
        self.protected = regions.to_vec();
    }

    fn keep(&mut self, cells: Keep) {
        self.kept = cells;
    }

    fn take_placed(&mut self) -> Vec<BlockPos> {
        std::mem::take(&mut self.placed)
    }
}
