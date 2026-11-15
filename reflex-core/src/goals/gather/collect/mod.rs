//! `collect <item> <count>`: pick up matching drops; with none on the ground, take the
//! item from an animal that yields it (shear a sheep, or kill), then pick up its drops.

mod hunt;
mod source;
#[cfg(test)]
mod tests;

pub use source::{sources, SOURCE_RADIUS};

use source::{choose, Choice, Memory, Method};

use hunt::{Hunt, HuntStep};

use crate::behaviours::common::heading_to;
use crate::channels::{Commands, LegsCmd, Speed};
use crate::goals::ports::{NavTarget, Navigator};
use crate::goals::task::{GoalCx, GoalTask, Plan, Step};
use crate::goals::travel::{Walk, Walker};
use crate::math::Vec3;

pub const DROP_RADIUS: f64 = 16.0;
/// Closer than this, and with a clear straight line, the drop is walked to directly.
const DIRECT: f64 = 6.0;
/// Ticks with no matching drop and no animal to take it from before the goal stops.
const EMPTY_LIMIT: u32 = 40;
/// Ticks to stand by after a kill or a shearing while the drops pop out and land.
const SETTLE: u32 = 15;

pub struct Collect {
    item: String,
    count: u32,
    baseline: Option<u32>,
    walker: Walker,
    empty: u32,
    hunt: Option<Hunt>,
    settle: u32,
    mem: Memory,
    sheared: u32,
    killed: u32,
}

impl Collect {
    pub fn new(item: String, count: u32) -> Self {
        Self {
            item,
            count: count.max(1),
            baseline: None,
            walker: Walker::default(),
            empty: 0,
            hunt: None,
            settle: 0,
            mem: Memory::default(),
            sheared: 0,
            killed: 0,
        }
    }

    /// Credit a finished hunt even when its drops were picked up before the hunt step ran.
    fn credit(&mut self, cx: &GoalCx) {
        let Some(h) = self.hunt.as_ref().filter(|h| h.finished(cx.snap())) else {
            return;
        };
        match h.method {
            Method::Shear => self.sheared += 1,
            Method::Kill => self.killed += 1,
        }
        self.hunt = None;
        self.settle = SETTLE;
    }

    fn how(&self) -> String {
        match (self.sheared, self.killed) {
            (0, 0) => String::new(),
            (s, 0) => format!(" (sheared {s})"),
            (0, k) => format!(" (killed {k})"),
            (s, k) => format!(" (sheared {s}, killed {k})"),
        }
    }

    /// No drops in range: work the current animal, or pick the next one.
    fn hunt(&mut self, cx: &mut GoalCx, got: u32) -> Step {
        let (item, n) = (&self.item, self.count);
        if self.hunt.is_none() {
            let shears = cx.world.item_slot("shears").is_some();
            let snap = cx.snap();
            match choose(item, &snap.entities, snap.me.pos, shears, &self.mem) {
                Choice::Take { id, method } => self.hunt = Some(Hunt::new(id, method)),
                other => {
                    self.walker.stop(cx);
                    if !cx.preempted {
                        self.empty += 1;
                    }
                    if self.empty <= EMPTY_LIMIT {
                        return Step::Bid(Plan::hold("goal_collect"));
                    }
                    let r = DROP_RADIUS;
                    return Step::Failed(match other {
                        Choice::NoneInRange(why) => format!(
                            "no {item} drops within {r:.0} blocks and {why}; collected {got} of {n}"
                        ),
                        _ => {
                            format!("no {item} drops within {r:.0} blocks; collected {got} of {n}")
                        }
                    });
                }
            }
        }
        self.empty = 0;
        let h = self.hunt.as_mut().expect("hunt chosen above");
        match h.step(cx, &mut self.walker) {
            HuntStep::Bid(p) => return Step::Bid(p),
            HuntStep::Done => {
                match h.method {
                    Method::Shear => self.sheared += 1,
                    Method::Kill => self.killed += 1,
                }
                self.settle = SETTLE;
            }
            HuntStep::GiveUp(why) => {
                self.mem.avoid.push(h.id);
                cx.blocked(why);
            }
            HuntStep::ShearFailed => {
                self.mem.no_shear.push(h.id);
                cx.blocked("shearing the sheep did nothing; killing it instead");
            }
        }
        self.hunt = None;
        self.walker.stop(cx);
        Step::Bid(Plan::hold("goal_collect"))
    }
}

impl GoalTask for Collect {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        self.credit(cx);
        let have = cx.world.count_item(&self.item);
        let base = *self.baseline.get_or_insert(have);
        let got = have.saturating_sub(base);
        let (item, n) = (&self.item, self.count);
        if got >= n {
            return Step::Done(format!("collected {got} {item}{}", self.how()));
        }
        cx.progress(got, n, format!("collected {got}/{n} {item}"));
        if cx.world.free_slots() == 0 {
            return Step::Failed(format!("inventory full; collected {got} of {n} {item}"));
        }
        let me = cx.me_pos();
        let drops = cx.world.item_drops(Some(item), me, DROP_RADIUS);
        let Some(d) = drops
            .iter()
            .min_by(|a, b| a.pos.dist(me).total_cmp(&b.pos.dist(me)))
        else {
            if self.settle > 0 {
                if !cx.preempted {
                    self.settle -= 1;
                }
                return Step::Bid(Plan::hold("goal_collect"));
            }
            return self.hunt(cx, got);
        };
        self.empty = 0;
        self.settle = 0;
        if d.pos.dist(me) <= DIRECT && straight_clear(cx.snap(), me, d.pos) {
            self.walker.stop(cx);
            let legs = LegsCmd::toward(heading_to(me, d.pos), Speed::Walk);
            return Step::Bid(Plan::new("goal_collect", Commands::default().legs(legs)));
        }
        let t = NavTarget::Block {
            pos: d.pos.block(),
            radius: 1.0,
        };
        match self.walker.walk(cx, t, "goal_collect") {
            Walk::Moving(plan) => Step::Bid(plan),
            Walk::Arrived => Step::Bid(Plan::hold("goal_collect")),
            Walk::Failed(f) => Step::Failed(format!(
                "cannot reach {item} drops ({f:?}); collected {got} of {n}"
            )),
        }
    }

    fn stop(&mut self, _nav: &mut dyn Navigator) {
        self.walker.reset();
    }
}

/// The body fits along the straight line from `from` to `to` at the current feet level:
/// no step up, no block in the way. Live bug: the direct walk ran into a pillar between the
/// bot and the drop, and only the unstuck reflex's jumps ever moved it.
pub(in crate::goals::gather) fn straight_clear(
    snap: &crate::snapshot::Snapshot,
    from: Vec3,
    to: Vec3,
) -> bool {
    let flat = Vec3::new(to.x - from.x, 0.0, to.z - from.z);
    let steps = (flat.len() / 0.25).ceil().max(1.0) as u32;
    let feet = from.y.floor();
    (0..=steps).all(|i| {
        let p = from + flat * (i as f64 / steps as f64);
        let cell = Vec3::new(p.x, feet + 0.5, p.z).block();
        snap.blocks.get(cell).passable() && snap.blocks.get(cell.above()).passable()
    })
}
