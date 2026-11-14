use crate::behaviours::homeostasis::{best_food, eat_commands, EATING_TICKS};
use crate::channels::MainCmd;
use crate::goals::task::{GoalCx, GoalTask, Plan, Step};

/// Ticks after the first bite before an unchanged food level counts as a failure.
const CONFIRM_TICKS: u32 = EATING_TICKS + 30;

#[derive(Default)]
pub struct EatNow {
    food_before: Option<u8>,
    eating: Option<u32>,
}

impl GoalTask for EatNow {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        let me = &cx.snap().me;
        let before = *self.food_before.get_or_insert(me.food);
        if me.food > before {
            return Step::Done(format!("ate, food {before} -> {}", me.food));
        }
        if let Some(t) = self.eating {
            if t > CONFIRM_TICKS {
                return Step::Failed("ate but the food level did not rise".into());
            }
            if !cx.preempted {
                self.eating = Some(t + 1);
            }
        }
        let Some(slot) = best_food(me, false).or_else(|| best_food(me, true)) else {
            return Step::Failed("no food in the inventory".into());
        };
        let golden = best_food(me, false).is_none();
        if me.food >= 20 && !golden {
            return Step::Failed("not hungry (food is full)".into());
        }
        let c = eat_commands(me, slot);
        if c.main == Some(MainCmd::Use) {
            if !cx.preempted && self.eating.is_none() {
                self.eating = Some(0);
            }
            return Step::Bid(Plan::new("goal_eat", c).lock(EATING_TICKS));
        }
        Step::Bid(Plan::new("goal_eat", c))
    }
}
