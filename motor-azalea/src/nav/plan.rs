//! Path planning on a worker thread with Azalea's own A* (`calculate_path`), without its
//! path executor: the nodes come back to the goal layer, which steers through the reflex
//! selector so reflexes keep the legs whenever they need them.

use std::sync::{
    atomic::{AtomicI32, AtomicUsize, Ordering},
    mpsc::{self, Receiver, TryRecvError},
    Arc,
};

use azalea::{
    pathfinder::{
        calculate_path, custom_state::CustomPathfinderState, goals::RadiusGoal,
        mining::MiningCache, moves, player_pos_to_block_pos, positions::RelBlockPos,
        CalculatePathCtx, PathfinderOpts,
    },
    Client,
};
use reflex_core::math::{BlockPos, Vec3};

pub struct Planned {
    pub nodes: Vec<BlockPos>,
    pub partial: bool,
}

pub enum Poll {
    Idle,
    Busy,
    Ready(Planned),
    /// The search was superseded or its thread died.
    Lost,
}

#[derive(Default)]
pub struct Planner {
    rx: Option<Receiver<Option<Planned>>>,
    /// Shared with every search; bumping it makes a running search discard its result.
    generation: Arc<AtomicUsize>,
}

/// Deepest drop a planned path may take, kept equal to the edge guard's (R5) safe drop so
/// the planner never routes over a ledge the reflex will refuse to step off. Azalea's basic
/// moves fall up to 3 blocks; R5 allows 2 at the default caution.
static MAX_DROP: AtomicI32 = AtomicI32::new(2);

/// Called by the tick loop with `Gains::safe_drop()`.
pub fn set_max_drop(blocks: f64) {
    MAX_DROP.store(blocks.floor().clamp(0.0, 3.0) as i32, Ordering::Relaxed);
}

/// Azalea's basic moves without descents deeper than `MAX_DROP`.
fn safe_moves(ctx: &mut moves::MovesCtx, node: RelBlockPos) {
    let first = ctx.edges.len();
    moves::basic::basic_move(ctx, node);
    let max = MAX_DROP.load(Ordering::Relaxed).max(1);
    let mut i = first;
    while i < ctx.edges.len() {
        if node.y - ctx.edges[i].movement.target.y > max {
            ctx.edges.swap_remove(i);
        } else {
            i += 1;
        }
    }
}

fn az(v: Vec3) -> azalea::Vec3 {
    azalea::Vec3::new(v.x, v.y, v.z)
}

impl Planner {
    /// Starts a search from `from` toward anywhere within `radius` of `goal`. Mining is off
    /// (block breaking belongs to the goals) and only the basic moves are allowed: walk,
    /// diagonal, one-block ascend and descend, drop down no deeper than the edge guard allows. Parkour needs sprint-jump timing
    /// the waypoint steering does not have.
    pub fn start(&mut self, bot: &Client, from: Vec3, goal: Vec3, radius: f64) -> bool {
        self.cancel();
        let Ok(world_lock) = bot.world() else {
            return false;
        };
        let ctx = CalculatePathCtx {
            entity: bot.entity,
            start: player_pos_to_block_pos(az(from)),
            goal: Arc::new(RadiusGoal::new(az(goal), radius as f32)),
            world_lock,
            goto_id_atomic: self.generation.clone(),
            mining_cache: MiningCache::new(None),
            custom_state: CustomPathfinderState::default(),
            opts: PathfinderOpts::new()
                .allow_mining(false)
                .successors_fn(safe_moves),
        };
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("goal-path".into())
            .spawn(move || {
                let found = calculate_path(ctx).map(|ev| Planned {
                    nodes: ev
                        .path
                        .unwrap_or_default()
                        .iter()
                        .map(|e| {
                            let t = e.movement.target;
                            BlockPos::new(t.x, t.y, t.z)
                        })
                        .collect(),
                    partial: ev.is_partial,
                });
                let _ = tx.send(found);
            });
        if spawned.is_err() {
            return false;
        }
        self.rx = Some(rx);
        true
    }

    pub fn poll(&mut self) -> Poll {
        let Some(rx) = &self.rx else {
            return Poll::Idle;
        };
        match rx.try_recv() {
            Err(TryRecvError::Empty) => Poll::Busy,
            Err(TryRecvError::Disconnected) | Ok(None) => {
                self.rx = None;
                Poll::Lost
            }
            Ok(Some(p)) => {
                self.rx = None;
                Poll::Ready(p)
            }
        }
    }

    /// Abandons the running search; its thread finishes on its own and its result is dropped.
    pub fn cancel(&mut self) {
        if self.rx.take().is_some() {
            self.generation.fetch_add(1, Ordering::SeqCst);
        }
    }
}
