//! Following a planned route with the snapshot and block reads as the only inputs, so any
//! planner (the adapter's, the test fakes) shares the same steering. A hop that needs the
//! hands (blocks to break, a block to place) is worked off before the legs move on.

mod work;

pub use work::{face_point, DIG_LIMIT};

use std::collections::VecDeque;

use crate::goals::ports::{NavFailure, Steer};
use crate::goals::travel::route::{Hop, Terrain};
use crate::math::BlockPos;
use crate::snapshot::Snapshot;

/// Ticks without reaching a node before the path counts as stalled.
pub const STALL_TICKS: u32 = 100;
const REACHED_XZ: f64 = 0.35;
/// Nodes looked ahead for a shortcut (the body may skip a node on diagonals).
const LOOKAHEAD: usize = 3;
const SPRINT_RUN: usize = 5;

#[derive(Clone, Debug, Default)]
pub struct PathFollower {
    hops: VecDeque<Hop>,
    since_progress: u32,
    /// Ticks spent on the current dig or placement.
    work_ticks: u32,
    placed: Vec<BlockPos>,
}

impl PathFollower {
    /// A walking path through these feet positions.
    pub fn new(nodes: impl IntoIterator<Item = BlockPos>) -> Self {
        let mut prev = None;
        let hops = nodes
            .into_iter()
            .map(|n| {
                let h = Hop::walk(prev.unwrap_or(n), n);
                prev = Some(n);
                h
            })
            .collect();
        Self::route(hops)
    }

    pub fn route(hops: Vec<Hop>) -> Self {
        Self {
            hops: hops.into(),
            ..Self::default()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.hops.is_empty()
    }

    pub fn remaining(&self) -> usize {
        self.hops.len()
    }

    pub fn last(&self) -> Option<BlockPos> {
        self.hops.back().map(|h| h.to)
    }

    /// The next hop still has hand work (a dig or a placement) to finish.
    pub fn working(&self) -> bool {
        self.hops.front().is_some_and(|h| h.has_work())
    }

    /// Scaffold blocks this path placed since the last call.
    pub fn take_placed(&mut self) -> Vec<BlockPos> {
        std::mem::take(&mut self.placed)
    }

    fn reached(snap: &Snapshot, n: BlockPos) -> bool {
        let me = snap.me.pos;
        let c = n.feet_centre();
        (me - c).horizontal().len() < REACHED_XZ && (me.y - n.y as f64).abs() < 0.6
    }

    /// Next steering command; `None` once every hop is done.
    pub fn steer(
        &mut self,
        snap: &Snapshot,
        t: &dyn Terrain,
    ) -> Option<Result<Steer, NavFailure>> {
        // Skip ahead only over hops with no hand work left.
        let free = self
            .hops
            .iter()
            .take(LOOKAHEAD)
            .take_while(|h| !h.has_work())
            .count();
        let hit = (0..free).rev().find(|&i| Self::reached(snap, self.hops[i].to));
        if let Some(i) = hit {
            self.hops.drain(..=i);
            self.since_progress = 0;
        }
        let head = self.hops.front_mut()?;
        match work::step(head, snap, t, &mut self.work_ticks, &mut self.placed) {
            work::Outcome::Busy(s) => {
                self.since_progress = 0;
                return Some(Ok(s));
            }
            work::Outcome::Failed(f) => return Some(Err(f)),
            work::Outcome::Clear => self.work_ticks = 0,
        }
        if Self::reached(snap, head.to) {
            self.hops.pop_front();
            self.since_progress = 0;
            return self.steer(snap, t);
        }
        self.since_progress += 1;
        if self.since_progress > STALL_TICKS {
            return Some(Err(NavFailure::Stalled));
        }
        let next = head.to;
        let me = &snap.me;
        let near = (me.pos - next.feet_centre()).horizontal().len() < 1.5;
        let jump = next.y > me.pos.y.floor() as i32 && near && me.on_ground;
        let sprint = self.straight_run() >= SPRINT_RUN;
        Some(Ok(Steer::walk(next.feet_centre(), jump, sprint)))
    }

    /// Length of the run of work-free hops ahead that keep the first step's direction and
    /// level.
    fn straight_run(&self) -> usize {
        let mut run = 0;
        let mut dir = None;
        for h in &self.hops {
            let d = (h.to.x - h.from.x, h.to.y - h.from.y, h.to.z - h.from.z);
            if d == (0, 0, 0) && !h.has_work() {
                run += 1;
                continue;
            }
            if h.has_work() || d.1 != 0 || dir.is_some_and(|x| x != d) {
                break;
            }
            dir = Some(d);
            run += 1;
        }
        run
    }
}
