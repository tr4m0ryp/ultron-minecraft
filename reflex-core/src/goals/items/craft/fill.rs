//! Filling empty bottles at a water source, the way a player does it: stand at the water's
//! edge, look at the surface with the bottle in hand and use it, once per bottle. The server
//! casts the use from the eye along the gaze and fills at the first water source it meets.

use crate::channels::{Commands, LegsCmd, MainCmd};
use crate::goals::items::{equip, hand_sees};
use crate::goals::ports::NavTarget;
use crate::goals::task::{GoalCx, Plan};
use crate::goals::travel::{Walk, Walker};
use crate::math::{BlockPos, Vec3};

/// The empty bottle and what it becomes.
pub const BOTTLE: &str = "glass_bottle";
pub const FILLED: &str = "water_potion";
/// Eye to the water surface, at most (survival reach is 4.5).
const REACH: f64 = 4.0;
/// How near the walk ends; the edge of the water is within this of its middle.
const STAND: f64 = 2.5;
/// Ticks without a bottle filling before the water is given up on.
const FILL_TICKS: u32 = 80;

// One value per tick, never stored; boxing the plan would buy nothing.
#[allow(clippy::large_enum_variant)]
pub enum Filled {
    Bid(Plan),
    Done,
    Failed(String),
}

pub struct Filling {
    pos: BlockPos,
    want: u32,
    /// Water bottles held when the step began.
    base: Option<u32>,
    /// The count the wait is measured from, and the ticks waited on it.
    seen: u32,
    ticks: u32,
    at_water: bool,
}

impl Filling {
    pub fn new(pos: BlockPos, want: u32) -> Self {
        Self {
            pos,
            want: want.max(1),
            base: None,
            seen: 0,
            ticks: 0,
            at_water: false,
        }
    }

    fn at(&self) -> String {
        format!("water at {} {} {}", self.pos.x, self.pos.y, self.pos.z)
    }

    /// The middle of the water's surface: what the gaze rests on.
    fn surface(&self) -> Vec3 {
        self.pos.feet_centre() + Vec3::new(0.0, 0.85, 0.0)
    }

    /// Walk back to the water after a suspension.
    pub fn reset(&mut self) {
        self.at_water = false;
    }

    pub fn step(&mut self, cx: &mut GoalCx, walker: &mut Walker) -> Filled {
        let held = cx.world.count_item(FILLED);
        let base = *self.base.get_or_insert(held);
        let got = held.saturating_sub(base);
        if got >= self.want {
            walker.stop(cx);
            return Filled::Done;
        }
        if cx.world.count_item(BOTTLE) == 0 {
            return Filled::Failed(format!("no {BOTTLE} left to fill; filled {got}"));
        }
        if !self.at_water {
            let t = NavTarget::Block {
                pos: self.pos,
                radius: STAND,
            };
            return match walker.walk(cx, t, "goal_fill_walk") {
                Walk::Moving(p) => Filled::Bid(p),
                Walk::Failed(f) => Filled::Failed(format!("cannot reach the {}: {f:?}", self.at())),
                Walk::Arrived => {
                    self.at_water = true;
                    self.ticks = 0;
                    Filled::Bid(Plan::hold("goal_fill"))
                }
            };
        }
        let me = cx.snap().me.clone();
        let (eye, aim) = (me.eye(), self.surface());
        if eye.dist(aim) > REACH || !hand_sees(&*cx.world, eye, aim, &[self.pos]) {
            return Filled::Failed(format!("the {} is not in sight from its edge", self.at()));
        }
        if got != self.seen {
            self.seen = got;
            self.ticks = 0;
        }
        if !cx.preempted {
            self.ticks += 1;
        }
        if self.ticks > FILL_TICKS {
            return Filled::Failed(format!("the {} filled no bottle; filled {got}", self.at()));
        }
        let Some(slot) = cx.world.item_slot(BOTTLE) else {
            return Filled::Failed(format!("no {BOTTLE} left to fill; filled {got}"));
        };
        if let Some(c) = equip(&me, slot) {
            return Filled::Bid(Plan::new("goal_equip", c));
        }
        let c = Commands::default()
            .legs(LegsCmd::stop())
            .look_at(aim)
            .main(MainCmd::Use);
        Filled::Bid(Plan::new("goal_fill", c))
    }
}
