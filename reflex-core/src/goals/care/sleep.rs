use crate::channels::{Commands, LegsCmd, MainCmd};
use crate::goals::ports::{NavTarget, Navigator};
use crate::goals::task::{GoalCx, GoalTask, Plan, Step};
use crate::goals::travel::{Walk, Walker};
use crate::math::{BlockPos, Vec3};

/// "In view": beds further than this are not looked for.
pub const BED_RADIUS: i32 = 16;
const BED_REACH: f64 = 2.0;
/// Vanilla refuses to sleep with a monster this close.
const MONSTER_RADIUS: f64 = 8.0;
const CLICK_GAP: u32 = 10;
const MAX_CLICKS: u32 = 5;

pub struct SleepInBed {
    bed: Option<BlockPos>,
    walker: Walker,
    clicks: u32,
    wait: u32,
}

impl Default for SleepInBed {
    fn default() -> Self {
        Self::new()
    }
}

impl SleepInBed {
    pub fn new() -> Self {
        Self {
            bed: None,
            walker: Walker::default(),
            clicks: 0,
            wait: 0,
        }
    }
}

impl GoalTask for SleepInBed {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        if cx.world.sleeping() {
            return Step::Done("asleep in bed".into());
        }
        // By day a player still walks over and clicks the bed: that sets the spawn point.
        let day = !cx.snap().is_night;
        if day && self.clicks > 0 && self.wait == 0 {
            return Step::Failed(
                "can only sleep at night; clicked the bed by day, which sets the spawn point"
                    .into(),
            );
        }
        let bed = match self.bed {
            Some(b) => b,
            None => {
                let from = cx.me_pos().block();
                let Some(b) = cx.world.nearest_block("bed", from, BED_RADIUS, &[]) else {
                    return Step::Failed(format!("no bed within {BED_RADIUS} blocks"));
                };
                *self.bed.insert(b)
            }
        };
        let t = NavTarget::Block {
            pos: bed,
            radius: BED_REACH,
        };
        match self.walker.walk(cx, t, "goal_sleep_walk") {
            Walk::Arrived => {}
            Walk::Moving(p) => return Step::Bid(p),
            Walk::Failed(f) => return Step::Failed(format!("cannot reach the bed: {f:?}")),
        }
        let centre = bed.feet_centre() + Vec3::new(0.0, 0.3, 0.0);
        let face = Commands::default().legs(LegsCmd::stop()).look_at(centre);
        if cx.snap().nearest_hostile(MONSTER_RADIUS).is_some() {
            cx.blocked("monsters nearby");
            return Step::Bid(Plan::new("goal_sleep", face));
        }
        if self.wait > 0 {
            if !cx.preempted {
                self.wait -= 1;
            }
            return Step::Bid(Plan::new("goal_sleep", face));
        }
        if self.clicks >= MAX_CLICKS {
            return Step::Failed("the bed refused (occupied, obstructed or unsafe)".into());
        }
        if !cx.preempted {
            self.clicks += 1;
            self.wait = CLICK_GAP;
        }
        // PlaceOn is a right click on the block; on a bed that means lie down.
        Step::Bid(Plan::new("goal_sleep", face.main(MainCmd::PlaceOn(bed))))
    }

    fn stop(&mut self, _nav: &mut dyn Navigator) {
        self.walker.reset();
    }
}
