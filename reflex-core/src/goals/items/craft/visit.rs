//! One visit to a brewing stand or smithing table: walk into reach, face it, hand the job
//! to the adapter on a tick the goal holds its channels, and wait for the menu to finish.

use super::REACH;
use crate::goals::ports::{MenuStatus, NavTarget, StationJob};
use crate::goals::task::{GoalCx, Intent, Plan};
use crate::goals::travel::{Walk, Walker};
use crate::math::{BlockPos, Vec3};

/// Ticks a visit (open, clicks, close) may take.
const VISIT_TIMEOUT: u32 = 240;

// One value per tick, never stored; boxing the plan would buy nothing.
#[allow(clippy::large_enum_variant)]
pub enum Visited {
    Bid(Plan),
    Done,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq)]
enum Phase {
    Walk,
    Open,
    Busy(u32),
}

pub struct Visit {
    pub pos: BlockPos,
    /// What stands there, for messages: "brewing stand", "smithing table".
    what: &'static str,
    job: StationJob,
    phase: Phase,
}

impl Visit {
    pub fn new(pos: BlockPos, what: &'static str, job: StationJob) -> Self {
        Self {
            pos,
            what,
            job,
            phase: Phase::Walk,
        }
    }

    pub fn at(&self) -> String {
        format!(
            "{} at {} {} {}",
            self.what, self.pos.x, self.pos.y, self.pos.z
        )
    }

    /// The runner handed the job to the adapter.
    pub fn committed(&mut self) {
        if self.phase == Phase::Open {
            self.phase = Phase::Busy(0);
        }
    }

    /// Walk back to it after a suspension. A visit the adapter already runs is left to end.
    pub fn reset(&mut self) {
        if self.phase == Phase::Open {
            self.phase = Phase::Walk;
        }
    }

    pub fn step(&mut self, cx: &mut GoalCx, walker: &mut Walker, note: &'static str) -> Visited {
        match self.phase {
            Phase::Walk => {
                let t = NavTarget::Block {
                    pos: self.pos,
                    radius: REACH,
                };
                match walker.walk(cx, t, note) {
                    Walk::Moving(p) => Visited::Bid(p),
                    Walk::Failed(f) => {
                        Visited::Failed(format!("cannot reach the {}: {f:?}", self.at()))
                    }
                    Walk::Arrived => {
                        self.phase = Phase::Open;
                        Visited::Bid(Plan::hold(note))
                    }
                }
            }
            Phase::Open => {
                let face = self.pos.feet_centre() + Vec3::new(0.0, 0.5, 0.0);
                let intent = Intent::Station {
                    pos: self.pos,
                    job: self.job.clone(),
                };
                Visited::Bid(Plan::face(note, face, cx.snap()).intent(intent))
            }
            Phase::Busy(ticks) => match cx.world.station_status() {
                MenuStatus::Done => Visited::Done,
                MenuStatus::Failed(r) => Visited::Failed(format!("{}: {r}", self.at())),
                MenuStatus::Busy | MenuStatus::Idle => {
                    if ticks > VISIT_TIMEOUT {
                        return Visited::Failed(format!("the {} did not open", self.at()));
                    }
                    if !cx.preempted {
                        self.phase = Phase::Busy(ticks + 1);
                    }
                    Visited::Bid(Plan::still(note, cx.snap()))
                }
            },
        }
    }
}
