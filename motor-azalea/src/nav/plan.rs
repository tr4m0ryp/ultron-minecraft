//! Route planning on a worker thread with reflex-core's planner over a cached copy of the
//! client's chunks. The hops come back to the goal layer, which steers and works through
//! the reflex selector so reflexes keep the body whenever they need it.

use std::sync::{
    atomic::{AtomicI32, AtomicUsize, Ordering},
    mpsc::{self, Receiver, TryRecvError},
    Arc,
};

use azalea::Client;
use reflex_core::{
    goals::travel::route::{plan, Hop, RouteGoal, RouteOpts},
    math::BlockPos,
};

use super::terrain::PlanTerrain;

pub struct Planned {
    pub hops: Vec<Hop>,
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
/// the planner never routes over a ledge the reflex will refuse to step off.
static MAX_DROP: AtomicI32 = AtomicI32::new(2);

/// Called by the tick loop with `Gains::safe_drop()`.
pub fn set_max_drop(blocks: f64) {
    MAX_DROP.store(blocks.floor().clamp(0.0, 3.0) as i32, Ordering::Relaxed);
}

pub fn max_drop() -> i32 {
    MAX_DROP.load(Ordering::Relaxed).max(1)
}

impl Planner {
    /// Starts a search from the standing cell `from` toward `goal`.
    pub fn start(
        &mut self,
        bot: &Client,
        from: BlockPos,
        goal: RouteGoal,
        opts: RouteOpts,
    ) -> bool {
        self.cancel();
        let Ok(world) = bot.world() else {
            return false;
        };
        let menu = crate::world::inventory_menu(bot);
        let generation = self.generation.clone();
        let mine = generation.load(Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("goal-path".into())
            .spawn(move || {
                let terrain = PlanTerrain::new(world, from, menu);
                let r = plan(&terrain, from, &goal, &opts);
                if generation.load(Ordering::SeqCst) != mine {
                    return;
                }
                let _ = tx.send(Some(Planned {
                    hops: r.hops,
                    partial: r.partial,
                }));
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
