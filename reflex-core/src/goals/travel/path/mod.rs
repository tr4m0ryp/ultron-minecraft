use std::collections::VecDeque;

use crate::goals::ports::{NavFailure, Steer};
use crate::math::BlockPos;
use crate::snapshot::Snapshot;

/// Ticks without reaching a node before the path counts as stalled.
pub const STALL_TICKS: u32 = 100;
const REACHED_XZ: f64 = 0.35;
/// Nodes looked ahead for a shortcut (the body may skip a node on diagonals).
const LOOKAHEAD: usize = 3;
const SPRINT_RUN: usize = 5;

/// Follows a planned list of feet positions with the snapshot as the only input, so any
/// planner that yields block nodes (the Azalea adapter, tests) shares the same steering.
#[derive(Clone, Debug, Default)]
pub struct PathFollower {
    nodes: VecDeque<BlockPos>,
    since_progress: u32,
}

impl PathFollower {
    pub fn new(nodes: impl IntoIterator<Item = BlockPos>) -> Self {
        Self {
            nodes: nodes.into_iter().collect(),
            since_progress: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn remaining(&self) -> usize {
        self.nodes.len()
    }

    pub fn last(&self) -> Option<BlockPos> {
        self.nodes.back().copied()
    }

    fn reached(snap: &Snapshot, n: BlockPos) -> bool {
        let me = snap.me.pos;
        let c = n.feet_centre();
        (me - c).horizontal().len() < REACHED_XZ && (me.y - n.y as f64).abs() < 0.6
    }

    /// Next steering command; `None` once every node is reached.
    pub fn steer(&mut self, snap: &Snapshot) -> Option<Result<Steer, NavFailure>> {
        let hit = (0..self.nodes.len().min(LOOKAHEAD))
            .rev()
            .find(|&i| Self::reached(snap, self.nodes[i]));
        if let Some(i) = hit {
            self.nodes.drain(..=i);
            self.since_progress = 0;
        } else {
            self.since_progress += 1;
        }
        let next = *self.nodes.front()?;
        if self.since_progress > STALL_TICKS {
            return Some(Err(NavFailure::Stalled));
        }
        let me = &snap.me;
        let near = (me.pos - next.feet_centre()).horizontal().len() < 1.5;
        let jump = next.y > me.pos.y.floor() as i32 && near && me.on_ground;
        Some(Ok(Steer {
            waypoint: next.feet_centre(),
            jump,
            sprint: self.straight_run() >= SPRINT_RUN,
        }))
    }

    /// Length of the run of nodes ahead that keep the first step's direction and level.
    fn straight_run(&self) -> usize {
        let mut it = self.nodes.iter();
        let Some(&a) = it.next() else {
            return 0;
        };
        let Some(&b) = it.next() else {
            return 1;
        };
        let (dx, dy, dz) = (b.x - a.x, b.y - a.y, b.z - a.z);
        if dy != 0 {
            return 1;
        }
        let mut run = 2;
        let mut prev = b;
        for &n in it {
            if (n.x - prev.x, n.y - prev.y, n.z - prev.z) != (dx, dy, dz) {
                break;
            }
            run += 1;
            prev = n;
        }
        run
    }
}
