//! Flight-recorder side of the goal runner: a navigator wrapper that remembers the last
//! poll, and the watch that decides when a stall or failure is worth a dump.

use std::cell::Cell;
use std::rc::Rc;

use crate::channels::{LegsCmd, Speed};
use crate::goals::ports::{NavFailure, NavStatus, NavTarget, Navigator};
use crate::goals::travel::route::Region;
use crate::math::{BlockPos, Vec3};
use crate::snapshot::Snapshot;
use crate::trace::{GoalTick, NavCode};

/// Ticks a moving goal may stand still before it counts as stalled (3 s). Shorter than the
/// path follower's own 100-tick stall, so the dump explains the stall before it is reported.
pub const NO_PROGRESS_TICKS: u32 = 60;
/// Blocks the body must travel within that window to count as progress.
const PROGRESS_BLOCKS: f64 = 0.5;
/// Dumps are at least this far apart (5 s), so a retry loop prints once per episode.
pub const DUMP_GAP_TICKS: u64 = 100;

/// What the wrapper saw on the goal's last poll this tick.
#[derive(Clone, Copy, Debug, Default)]
pub struct Seen {
    pub nav: NavCode,
    pub flags: u8,
    pub wp: [i32; 3],
}

pub type SeenCell = Rc<Cell<Seen>>;

/// A legs command that walks somewhere, as opposed to a hold.
pub fn asks_to_move(l: &LegsCmd) -> bool {
    l.speed != Speed::Stop && l.heading.horizontal().len() > 1e-6
}

pub fn code_of(f: &NavFailure) -> NavCode {
    match f {
        NavFailure::Unreachable => NavCode::Unreachable,
        NavFailure::TargetLost => NavCode::TargetLost,
        NavFailure::Stalled => NavCode::Stalled,
        NavFailure::NoScaffold => NavCode::NoScaffold,
    }
}

/// Delegates everything and records the status of each `poll`.
pub struct TracedNav {
    inner: Box<dyn Navigator>,
    seen: SeenCell,
}

impl TracedNav {
    pub fn new(inner: Box<dyn Navigator>, seen: SeenCell) -> Self {
        Self { inner, seen }
    }
}

impl Navigator for TracedNav {
    fn request(&mut self, target: NavTarget, snap: &Snapshot) {
        self.inner.request(target, snap);
    }

    fn poll(&mut self, snap: &Snapshot) -> NavStatus {
        let status = self.inner.poll(snap);
        let mut s = Seen::default();
        match &status {
            NavStatus::Idle => s.nav = NavCode::Idle,
            NavStatus::Planning => s.nav = NavCode::Planning,
            NavStatus::Arrived => s.nav = NavCode::Arrived,
            NavStatus::Failed(f) => s.nav = code_of(f),
            NavStatus::Moving(st) => {
                s.nav = if st.work.is_some() {
                    NavCode::Work
                } else {
                    NavCode::Moving
                };
                s.flags = u8::from(st.jump) | u8::from(st.sprint) << 1 | u8::from(st.sneak) << 2;
                let b: BlockPos = st.waypoint.block();
                s.wp = [b.x, b.y, b.z];
            }
        }
        self.seen.set(s);
        status
    }

    fn pause(&mut self) {
        self.inner.pause();
    }

    fn cancel(&mut self) {
        self.inner.cancel();
    }

    fn protect(&mut self, regions: &[Region]) {
        self.inner.protect(regions);
    }

    fn take_placed(&mut self) -> Vec<BlockPos> {
        self.inner.take_placed()
    }
}

/// Counts ticks of "walking but not getting anywhere" and rate-limits dumps. A reflex taking
/// the legs for a tick (and the re-plan that follows) does not reset the count: that is
/// exactly how a body can scrape a wall for minutes without the navigator ever stalling.
#[derive(Default)]
pub struct StallWatch {
    goal_id: u64,
    anchor: Option<Vec3>,
    still: u32,
    last_dump: Option<u64>,
}

impl StallWatch {
    fn reset(&mut self) {
        self.anchor = None;
        self.still = 0;
    }

    /// Feed one goal tick; returns a dump reason when the goal just stalled.
    pub fn observe(&mut self, g: &GoalTick, pos: Vec3) -> Option<String> {
        if g.goal_id != self.goal_id {
            self.goal_id = g.goal_id;
            self.reset();
        }
        if g.goal_id == 0 {
            self.reset();
            return None;
        }
        if !g.moving {
            // A reflex holding the legs for a tick does not end the episode; a goal that
            // holds, digs, plans or has arrived does.
            if !g.preempted {
                self.reset();
            }
            return None;
        }
        if !matches!(g.nav, NavCode::Moving | NavCode::None) {
            self.reset();
            return None;
        }
        let a = *self.anchor.get_or_insert(pos);
        if pos.dist(a) >= PROGRESS_BLOCKS {
            self.anchor = Some(pos);
            self.still = 0;
            return None;
        }
        self.still += 1;
        (self.still == NO_PROGRESS_TICKS).then(|| {
            format!(
                "#{} {} no progress for {} ticks while moving",
                g.goal_id, g.label, NO_PROGRESS_TICKS
            )
        })
    }

    /// Whether a dump may be sent now; records it when it may.
    pub fn allow(&mut self, tick: u64) -> bool {
        if self.last_dump.is_some_and(|t| tick < t + DUMP_GAP_TICKS) {
            return false;
        }
        self.last_dump = Some(tick);
        true
    }
}
