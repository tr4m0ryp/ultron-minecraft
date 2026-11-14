use super::eat::{best_food, eat_commands, food_servings, EATING_TICKS};
use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::channels::MainCmd;
use crate::events::EventClass;

/// H1: food at or below hunger_floor with no hostile within 12 blocks: eat the best food.
/// Emergency at food 6 or less (sprinting impossible, so fleeing is too): P1, eats unless
/// a hostile is within 4.
#[derive(Default)]
pub struct Hunger {
    low_stock_reported: bool,
}

const EMERGENCY_FOOD: u8 = 6;
const LOW_STOCK: u32 = 5;

impl Behaviour for Hunger {
    fn id(&self) -> &'static str {
        "H1"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let me = &ctx.snap.me;
        let stock = food_servings(me);
        if stock < LOW_STOCK && !self.low_stock_reported {
            ctx.emit(
                ctx.report(EventClass::Digest, "food_stock_low", "")
                    .magnitude(stock as f32),
            );
        }
        self.low_stock_reported = stock < LOW_STOCK;

        let floor = ctx.knobs.gains.hunger_floor;
        if me.food > floor {
            return None;
        }
        let emergency = me.food <= EMERGENCY_FOOD;
        let safe_radius = if emergency { 4.0 } else { 12.0 };
        if ctx.snap.nearest_hostile(safe_radius).is_some() {
            return None;
        }
        let slot = best_food(me, false)?;
        let cmds = eat_commands(me, slot);
        let eating = cmds.main == Some(MainCmd::Use);
        let priority = if emergency {
            Priority::P1
        } else {
            Priority::P3
        };
        let urgency = 1.0 - me.food as f32 / (floor.max(1) as f32 + 1.0);
        let bid = Bid::new(priority, urgency, "eat", cmds);
        Some(if eating { bid.lock(EATING_TICKS) } else { bid })
    }
}
