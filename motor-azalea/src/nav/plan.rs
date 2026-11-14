//! Path planning on a worker thread with Azalea's own A* (`calculate_path`), without its
//! path executor: the nodes come back to the goal layer, which steers through the reflex
//! selector so reflexes keep the legs whenever they need them.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc::{self, Receiver, TryRecvError},
    Arc,
};

use azalea::{
    pathfinder::{
        calculate_path, custom_state::CustomPathfinderState, goals::RadiusGoal,
        mining::MiningCache, moves, player_pos_to_block_pos, CalculatePathCtx, PathfinderOpts,
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

fn az(v: Vec3) -> azalea::Vec3 {
    azalea::Vec3::new(v.x, v.y, v.z)
}

impl Planner {
    /// Starts a search from `from` toward anywhere within `radius` of `goal`. Mining is off
    /// (block breaking belongs to the goals) and only the basic moves are allowed: walk,
    /// diagonal, one-block ascend and descend, drop down. Parkour needs sprint-jump timing
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
                .successors_fn(moves::basic::basic_move),
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
