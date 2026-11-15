use crate::goals::ports::{NavTarget, Navigator};
use crate::goals::spec::Person;
use crate::goals::task::{GoalCx, GoalTask, Intent, Plan, Step};
use crate::goals::travel::{person_label, resolve_person, Walk, Walker};
use crate::math::Vec3;

/// Toss from this close so the stack lands at the person's feet.
const HAND_OVER: f64 = 2.5;
const LOST_LIMIT: u32 = 100;
const CONFIRM_TICKS: u32 = 40;
/// Ticks spent turning toward the person before the toss, so it flies their way.
const AIM_TICKS: u32 = 3;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Approach,
    Toss(u32),
    Confirm { before: u32, ticks: u32 },
}

pub struct Give {
    who: Person,
    item: String,
    count: u32,
    phase: Phase,
    walker: Walker,
    lost: u32,
    held_before: u32,
}

impl Give {
    pub fn new(who: Person, item: String, count: u32) -> Self {
        Self {
            who,
            item,
            count: count.max(1),
            phase: Phase::Approach,
            walker: Walker::default(),
            lost: 0,
            held_before: 0,
        }
    }
}

impl GoalTask for Give {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        let (item, n) = (self.item.clone(), self.count);
        let have = cx.world.count_item(&item);
        if let Phase::Confirm { before, ticks } = self.phase {
            if have + n <= before {
                return Step::Done(format!("gave {n} {item} to {}", person_label(&self.who)));
            }
            if ticks > CONFIRM_TICKS {
                return Step::Failed(format!("tossed {item} but the inventory did not change"));
            }
            if !cx.preempted {
                self.phase = Phase::Confirm {
                    before,
                    ticks: ticks + 1,
                };
            }
            return Step::Bid(Plan::hold("goal_give"));
        }
        if have < n {
            return Step::Failed(if have == 0 {
                format!("no {item} to give")
            } else {
                format!("only {have} {item}, asked to give {n}")
            });
        }
        let Some((id, pos, name)) = resolve_person(cx, &self.who) else {
            self.lost += 1;
            if self.lost > LOST_LIMIT {
                return Step::Failed(format!("{} not visible", person_label(&self.who)));
            }
            cx.stuck(format!("{} not visible", person_label(&self.who)));
            return Step::Idle;
        };
        self.lost = 0;
        if self.phase == Phase::Approach {
            let t = NavTarget::Entity {
                id,
                distance: HAND_OVER,
            };
            match self.walker.walk(cx, t, "goal_give_walk") {
                Walk::Arrived => self.phase = Phase::Toss(0),
                Walk::Moving(p) => return Step::Bid(p),
                Walk::Failed(f) => return Step::Failed(format!("cannot reach {name}: {f:?}")),
            }
        }
        if cx.me_pos().dist(pos) > HAND_OVER + 1.5 {
            self.phase = Phase::Approach;
            return Step::Bid(Plan::hold("goal_give"));
        }
        self.held_before = have;
        let chest = pos + Vec3::new(0.0, 1.0, 0.0);
        let plan = Plan::face("goal_give", chest, cx.snap());
        let Phase::Toss(aimed) = self.phase else {
            return Step::Bid(plan);
        };
        if aimed < AIM_TICKS {
            if !cx.preempted {
                self.phase = Phase::Toss(aimed + 1);
            }
            return Step::Bid(plan);
        }
        Step::Bid(plan.intent(Intent::Toss { item, count: n }))
    }

    fn committed(&mut self) {
        if let Phase::Toss(_) = self.phase {
            self.phase = Phase::Confirm {
                before: self.held_before,
                ticks: 0,
            };
        }
    }

    fn stop(&mut self, _nav: &mut dyn Navigator) {
        self.walker.reset();
        if let Phase::Toss(_) = self.phase {
            self.phase = Phase::Approach;
        }
    }
}
