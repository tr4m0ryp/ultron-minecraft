//! A* over standing cells with the player's moves. Library-free so the same search runs in
//! tests on a grid and live on the adapter's world.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use super::cell::{Cell, Terrain};
use super::hop::{Hop, HopKind};
use super::moves::{Edge, Moves, RouteOpts, WALK};
use crate::math::{BlockPos, Vec3};

/// Cells above the head scanned for open sky.
pub const SKY_SCAN: i32 = 48;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RouteGoal {
    /// Stand within `radius` of `centre` (measured from the body's middle).
    Near { centre: Vec3, radius: f64 },
    /// Stand anywhere with open sky overhead; `hint` (the last place known to have it)
    /// steers the search.
    Sky { hint: BlockPos },
    /// Stand level with `block` (at feet or head height) in a cardinal neighbour column,
    /// where a player mines it from the side.
    Beside { block: BlockPos },
}

impl RouteGoal {
    pub fn reached(&self, t: &dyn Terrain, n: BlockPos) -> bool {
        match *self {
            RouteGoal::Near { centre, radius } => {
                (n.feet_centre() + Vec3::new(0.0, 0.5, 0.0)).dist(centre) <= radius.max(0.5)
            }
            RouteGoal::Sky { .. } => open_sky(t, n),
            RouteGoal::Beside { block } => {
                let (dx, dz) = ((block.x - n.x).abs(), (block.z - n.z).abs());
                dx + dz == 1 && (block.y == n.y || block.y == n.y + 1)
            }
        }
    }

    fn estimate(&self, n: BlockPos) -> f32 {
        let d = match *self {
            RouteGoal::Near { centre, radius } => {
                ((n.feet_centre() + Vec3::new(0.0, 0.5, 0.0)).dist(centre) - radius).max(0.0)
            }
            RouteGoal::Sky { hint } => n.feet_centre().dist(hint.feet_centre()) * 0.3,
            RouteGoal::Beside { block } => (n.feet_centre().dist(block.feet_centre()) - 1.0).max(0.0),
        };
        d as f32 * WALK
    }
}

/// Nothing but open air (or the world's top) above the head.
pub fn open_sky(t: &dyn Terrain, n: BlockPos) -> bool {
    (2..=SKY_SCAN).all(|dy| matches!(t.cell(n.offset(0, dy, 0)), Cell::Open | Cell::Unknown))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub hops: Vec<Hop>,
    /// The goal was not reached; the route ends at the closest point found.
    pub partial: bool,
    pub expanded: usize,
}

impl Route {
    pub fn digs(&self) -> usize {
        self.hops.iter().map(|h| h.dig.len()).sum()
    }
    pub fn places(&self) -> usize {
        self.hops.iter().filter(|h| h.place.is_some()).count()
    }
}

struct Rec {
    g: f32,
    hop: Option<Hop>,
    placed: bool,
    left: u32,
    closed: bool,
}

#[derive(PartialEq)]
struct Open(f32, f32, BlockPos);
impl Eq for Open {}
impl Ord for Open {
    fn cmp(&self, o: &Self) -> Ordering {
        o.0.total_cmp(&self.0).then(o.1.total_cmp(&self.1))
    }
}
impl PartialOrd for Open {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// Plans from the standing cell `start` toward `goal`.
pub fn plan(t: &dyn Terrain, start: BlockPos, goal: &RouteGoal, o: &RouteOpts) -> Route {
    let moves = Moves { t, o };
    let mut recs: HashMap<BlockPos, Rec> = HashMap::new();
    let mut heap = BinaryHeap::new();
    recs.insert(
        start,
        Rec {
            g: 0.0,
            hop: None,
            placed: false,
            left: o.scaffold,
            closed: false,
        },
    );
    heap.push(Open(goal.estimate(start), 0.0, start));
    let (mut best, mut best_h) = (start, goal.estimate(start));
    let mut expanded = 0;
    let mut edges: Vec<Edge> = Vec::new();
    let mut found = None;
    while let Some(Open(_, g, n)) = heap.pop() {
        let Some(rec) = recs.get_mut(&n) else {
            continue;
        };
        if rec.closed || g > rec.g {
            continue;
        }
        rec.closed = true;
        let (placed, left) = (rec.placed, rec.left);
        if goal.reached(t, n) {
            found = Some(n);
            break;
        }
        expanded += 1;
        if expanded > o.max_nodes {
            break;
        }
        let h = goal.estimate(n);
        if h < best_h {
            (best, best_h) = (n, h);
        }
        edges.clear();
        moves.from(n, placed, left, &mut edges);
        for e in edges.drain(..) {
            let to = e.hop.to;
            let g2 = g + e.cost;
            let spends = e.hop.place.is_some();
            let better = recs.get(&to).is_none_or(|r| !r.closed && g2 < r.g);
            if better {
                heap.push(Open(g2 + goal.estimate(to), g2, to));
                recs.insert(
                    to,
                    Rec {
                        g: g2,
                        placed: spends && e.hop.kind != HopKind::Walk,
                        left: left - u32::from(spends),
                        hop: Some(e.hop),
                        closed: false,
                    },
                );
            }
        }
    }
    let (end, partial) = match found {
        Some(n) => (n, false),
        None => (best, true),
    };
    let mut hops = Vec::new();
    let mut at = end;
    while let Some(h) = recs.get(&at).and_then(|r| r.hop.clone()) {
        at = h.from;
        hops.push(h);
    }
    hops.reverse();
    Route {
        hops,
        partial,
        expanded,
    }
}
