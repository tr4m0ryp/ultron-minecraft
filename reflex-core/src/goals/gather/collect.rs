use crate::behaviours::common::heading_to;
use crate::channels::{Commands, LegsCmd, Speed};
use crate::goals::ports::{NavTarget, Navigator};
use crate::goals::task::{GoalCx, GoalTask, Plan, Step};
use crate::goals::travel::{Walk, Walker};
use crate::math::Vec3;

pub const DROP_RADIUS: f64 = 16.0;
/// Closer than this, and with a clear straight line, the drop is walked to directly.
const DIRECT: f64 = 6.0;
/// Ticks with no matching drop in range before the goal stops.
const EMPTY_LIMIT: u32 = 40;

pub struct Collect {
    item: String,
    count: u32,
    baseline: Option<u32>,
    walker: Walker,
    empty: u32,
}

impl Collect {
    pub fn new(item: String, count: u32) -> Self {
        Self {
            item,
            count: count.max(1),
            baseline: None,
            walker: Walker::default(),
            empty: 0,
        }
    }
}

impl GoalTask for Collect {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        let have = cx.world.count_item(&self.item);
        let base = *self.baseline.get_or_insert(have);
        let got = have.saturating_sub(base);
        let (item, n) = (&self.item, self.count);
        if got >= n {
            return Step::Done(format!("collected {got} {item}"));
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
            self.walker.stop(cx);
            if !cx.preempted {
                self.empty += 1;
            }
            if self.empty > EMPTY_LIMIT {
                return Step::Failed(format!(
                    "no {item} drops within {DROP_RADIUS:.0} blocks; collected {got} of {n}"
                ));
            }
            return Step::Bid(Plan::hold("goal_collect"));
        };
        self.empty = 0;
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
pub(super) fn straight_clear(snap: &crate::snapshot::Snapshot, from: Vec3, to: Vec3) -> bool {
    let flat = Vec3::new(to.x - from.x, 0.0, to.z - from.z);
    let steps = (flat.len() / 0.25).ceil().max(1.0) as u32;
    let feet = from.y.floor();
    (0..=steps).all(|i| {
        let p = from + flat * (i as f64 / steps as f64);
        let cell = Vec3::new(p.x, feet + 0.5, p.z).block();
        snap.blocks.get(cell).passable() && snap.blocks.get(cell.above()).passable()
    })
}
