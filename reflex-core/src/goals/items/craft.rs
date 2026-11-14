use crate::goals::ports::{MenuStatus, NavTarget, Navigator};
use crate::goals::task::{GoalCx, GoalTask, Intent, Plan, Step};
use crate::goals::travel::{Walk, Walker};
use crate::math::{BlockPos, Vec3};

pub const TABLE_RADIUS: i32 = 16;
pub const CRAFT_TIMEOUT_TICKS: u32 = 200;
const TABLE_REACH: f64 = 3.5;
const MAX_ROUNDS: u32 = 3;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Check,
    Approach(Option<BlockPos>),
    Craft(Option<BlockPos>),
    Waiting(u32),
}

pub struct Craft {
    item: String,
    count: u32,
    baseline: Option<u32>,
    crafts: u32,
    rounds: u32,
    phase: Phase,
    walker: Walker,
}

impl Craft {
    pub fn new(item: String, count: u32) -> Self {
        Self {
            item,
            count: count.max(1),
            baseline: None,
            crafts: 0,
            rounds: 0,
            phase: Phase::Check,
            walker: Walker::default(),
        }
    }

    /// Recipe, ingredient and table checks. Recursive crafting is the brain's job: a
    /// missing intermediate is reported, not crafted.
    fn check(&mut self, cx: &mut GoalCx, missing_n: u32) -> Result<Option<BlockPos>, String> {
        let item = &self.item;
        let r = cx
            .world
            .recipe(item)
            .ok_or(format!("no known recipe for {item}"))?;
        self.crafts = missing_n.div_ceil(r.yields.max(1));
        let short: Vec<String> = r
            .ingredients
            .iter()
            .filter_map(|(ing, n)| {
                let need = n * self.crafts;
                let have = cx.world.count_item(ing);
                (have < need).then(|| format!("{} more {ing}", need - have))
            })
            .collect();
        if !short.is_empty() {
            return Err(format!(
                "missing ingredients for {item}: {}",
                short.join(", ")
            ));
        }
        if cx.world.free_slots() == 0 {
            return Err(format!("inventory full; no room for {item}"));
        }
        if !r.needs_table {
            return Ok(None);
        }
        let from = cx.me_pos().block();
        cx.world
            .nearest_block("crafting_table", from, TABLE_RADIUS, &[])
            .map(Some)
            .ok_or(format!(
                "{item} needs a crafting table within {TABLE_RADIUS} blocks"
            ))
    }
}

impl GoalTask for Craft {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        let have = cx.world.count_item(&self.item);
        let base = *self.baseline.get_or_insert(have);
        let got = have.saturating_sub(base);
        if got >= self.count {
            return Step::Done(format!("crafted {got} {}", self.item));
        }
        match self.phase {
            Phase::Check => {
                self.rounds += 1;
                if self.rounds > MAX_ROUNDS {
                    return Step::Failed(format!(
                        "crafted only {got} of {} {}",
                        self.count, self.item
                    ));
                }
                match self.check(cx, self.count - got) {
                    Ok(table) => self.phase = Phase::Approach(table),
                    Err(e) => return Step::Failed(e),
                }
                Step::Bid(Plan::hold("goal_craft"))
            }
            Phase::Approach(None) => {
                self.phase = Phase::Craft(None);
                Step::Bid(Plan::hold("goal_craft"))
            }
            Phase::Approach(Some(t)) => {
                let target = NavTarget::Block {
                    pos: t,
                    radius: TABLE_REACH,
                };
                match self.walker.walk(cx, target, "goal_craft_walk") {
                    Walk::Arrived => {
                        self.phase = Phase::Craft(Some(t));
                        Step::Bid(Plan::hold("goal_craft"))
                    }
                    Walk::Moving(p) => Step::Bid(p),
                    Walk::Failed(f) => {
                        Step::Failed(format!("cannot reach the crafting table: {f:?}"))
                    }
                }
            }
            Phase::Craft(table) => {
                let intent = Intent::Craft {
                    item: self.item.clone(),
                    crafts: self.crafts,
                    table,
                };
                let plan = match table {
                    Some(t) => Plan::face(
                        "goal_craft",
                        t.feet_centre() + Vec3::new(0.0, 0.5, 0.0),
                        cx.snap(),
                    ),
                    None => Plan::still("goal_craft", cx.snap()),
                };
                Step::Bid(plan.intent(intent))
            }
            Phase::Waiting(ticks) => match cx.world.craft_status() {
                MenuStatus::Done => {
                    self.phase = Phase::Check;
                    Step::Bid(Plan::hold("goal_craft"))
                }
                MenuStatus::Failed(r) => {
                    Step::Failed(format!("crafting {} failed: {r}", self.item))
                }
                MenuStatus::Busy | MenuStatus::Idle => {
                    if ticks > CRAFT_TIMEOUT_TICKS {
                        return Step::Failed(format!("crafting {} timed out", self.item));
                    }
                    if !cx.preempted {
                        self.phase = Phase::Waiting(ticks + 1);
                    }
                    Step::Bid(Plan::still("goal_craft", cx.snap()))
                }
            },
        }
    }

    fn committed(&mut self) {
        if let Phase::Craft(_) = self.phase {
            self.phase = Phase::Waiting(0);
        }
    }

    fn stop(&mut self, _nav: &mut dyn Navigator) {
        self.walker.reset();
        self.phase = Phase::Check;
        self.rounds = 0;
    }
}
