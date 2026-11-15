//! Sleeping in a bed the body can see. It walks to a spot whose eye has a clear line to
//! the bed and clicks from there; a bed behind a wall is walked around to, and a bed
//! walled in on every side is refused. Live, the body slept in a bed inside a shelter
//! while standing outside its wall: the server takes that click, a player cannot make it.

use crate::channels::{Commands, LegsCmd, MainCmd};
use crate::goals::items::hand_sees;
use crate::goals::items::place::{eye_at, free, onto_centre};
use crate::goals::ports::{NavTarget, Navigator};
use crate::goals::task::{GoalCx, GoalTask, Plan, Step};
use crate::goals::travel::{Walk, Walker};
use crate::math::{BlockPos, Vec3};

/// "In view": beds further than this are not looked for.
pub const BED_RADIUS: i32 = 16;
/// The server refuses a bed more than 3 blocks off along x or z, or 2 up or down.
const BED_REACH_XZ: f64 = 2.5;
const BED_REACH_Y: f64 = 2.0;
/// Standing spots are looked for this far around the bed.
const SPOT_RADIUS: i32 = 3;
/// Standing spots tried before the bed counts as out of sight.
const MAX_SPOTS: usize = 3;
/// Ticks the body may take to step onto the middle of a spot.
const SETTLE_TICKS: u32 = 30;
/// Vanilla refuses to sleep with a monster this close.
const MONSTER_RADIUS: f64 = 8.0;
const CLICK_GAP: u32 = 10;
const MAX_CLICKS: u32 = 5;
const SIDES: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

#[derive(Default)]
pub struct SleepInBed {
    bed: Option<BlockPos>,
    walker: Walker,
    clicks: u32,
    wait: u32,
    spot: Option<BlockPos>,
    /// Ticks spent stepping onto the middle of the current spot.
    settle: u32,
    /// Spots that turned out not to see the bed once the body stood there.
    tried: Vec<BlockPos>,
}

impl SleepInBed {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Where a click on the bed cell `c` lands: the middle of its top.
fn top(c: BlockPos) -> Vec3 {
    c.feet_centre() + Vec3::new(0.0, 0.5, 0.0)
}

/// The cell of `bed` and its other half (any bed cell beside it).
fn halves(cx: &GoalCx, bed: BlockPos) -> Vec<BlockPos> {
    let is_bed = |p: &BlockPos| cx.world.block_at(*p).is_some_and(|k| k.ends_with("_bed"));
    let beside = SIDES.iter().map(|&(x, z)| bed.offset(x, 0, z));
    std::iter::once(bed).chain(beside.filter(is_bed)).collect()
}

/// The half an eye at `eye` can click: within the server's reach and in plain sight.
fn seen(cx: &GoalCx, eye: Vec3, halves: &[BlockPos]) -> Option<BlockPos> {
    halves.iter().copied().find(|&h| {
        let d = top(h) - eye;
        d.x.abs() <= BED_REACH_XZ
            && d.z.abs() <= BED_REACH_XZ
            && d.y.abs() <= BED_REACH_Y + 1.62
            && hand_sees(&*cx.world, eye, top(h), halves)
    })
}

/// The nearest standing spot that sees the bed, skipping `tried`.
fn spot(cx: &GoalCx, halves: &[BlockPos], tried: &[BlockPos]) -> Option<BlockPos> {
    let me = cx.me_pos();
    let r = SPOT_RADIUS;
    let cells = (-1..=1).flat_map(|dy| {
        (-r..=r).flat_map(move |dx| (-r..=r).map(move |dz| halves[0].offset(dx, dy, dz)))
    });
    cells
        .filter(|f| !tried.contains(f) && !halves.contains(f))
        .filter(|&f| free(cx, f) && free(cx, f.above()) && !free(cx, f.below()))
        .filter(|&f| seen(cx, eye_at(f), halves).is_some())
        .min_by(|a, b| {
            let d = |p: &BlockPos| me.dist(p.feet_centre());
            d(a).total_cmp(&d(b))
        })
}

impl SleepInBed {
    /// Walk to a spot that sees the bed.
    fn approach(&mut self, cx: &mut GoalCx, bed: BlockPos, halves: &[BlockPos]) -> Step {
        let at = format!("{} {} {}", bed.x, bed.y, bed.z);
        let target = match self.spot {
            Some(s) => s,
            None if self.tried.len() >= MAX_SPOTS => {
                return Step::Failed(format!("cannot get the bed at {at} in sight"));
            }
            None => match spot(cx, halves, &self.tried) {
                Some(s) => *self.spot.insert(s),
                None => {
                    return Step::Failed(format!(
                        "the bed at {at} is walled in: nowhere to stand that sees it"
                    ));
                }
            },
        };
        let t = NavTarget::Block {
            pos: target,
            radius: 0.5,
        };
        match self.walker.walk(cx, t, "goal_sleep_walk") {
            Walk::Moving(p) => Step::Bid(p),
            // There, and the bed is still hidden: onto the middle of the spot, then
            // another spot.
            Walk::Arrived => {
                let centre = onto_centre(cx.me_pos(), target);
                if let Some(c) = centre.filter(|_| self.settle < SETTLE_TICKS) {
                    self.settle += u32::from(!cx.preempted);
                    return Step::Bid(Plan::new("goal_sleep_walk", c));
                }
                self.settle = 0;
                self.tried.push(target);
                self.spot = None;
                Step::Bid(Plan::hold("goal_sleep"))
            }
            Walk::Failed(f) => Step::Failed(format!("cannot reach the bed: {f:?}")),
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
        let halves = halves(cx, bed);
        let me = cx.snap().me.clone();
        let visible = me.on_ground.then(|| seen(cx, me.eye(), &halves)).flatten();
        let Some(half) = visible else {
            return self.approach(cx, bed, &halves);
        };
        self.walker.stop(cx);
        let face = Commands::default()
            .legs(LegsCmd::stop())
            .look_at(top(half) - Vec3::new(0.0, 0.2, 0.0));
        if cx.snap().nearest_hostile(MONSTER_RADIUS).is_some() {
            cx.stuck("monsters nearby");
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
        Step::Bid(Plan::new("goal_sleep", face.main(MainCmd::PlaceOn(half))))
    }

    fn stop(&mut self, _nav: &mut dyn Navigator) {
        self.walker.reset();
        self.spot = None;
    }
}
