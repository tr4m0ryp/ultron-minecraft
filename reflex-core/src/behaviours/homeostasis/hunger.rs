use super::eat::{best_food, eat_commands, food_servings, meal_is_safe, EATING_TICKS};
use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::channels::MainCmd;
use crate::events::EventClass;

/// H1: food at or below hunger_floor with no hostile within 12 blocks: eat the best food.
/// Emergency at food 6 or less (sprinting impossible, so fleeing is too): P1, eats unless
/// a hostile is within 4. Never starts a meal in an archer's line of fire (`meal_is_safe`).
#[derive(Default)]
pub struct Hunger {
    low_stock_reported: bool,
    /// Hungry with nothing to eat was told to thought; `Some(true)` once it was an emergency.
    no_food_told: Option<bool>,
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
        self.tell_no_food(ctx, stock, floor);
        if me.food > floor {
            return None;
        }
        let emergency = me.food <= EMERGENCY_FOOD;
        let safe_radius = if emergency { 4.0 } else { 12.0 };
        if ctx.snap.nearest_hostile(safe_radius).is_some() || !meal_is_safe(ctx.snap) {
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

impl Hunger {
    /// Hungry with no food, the reflex has nothing to do and the body would sit idle: wake
    /// thought with what a player does next, once, and again when it turns into an emergency.
    fn tell_no_food(&mut self, ctx: &Ctx, stock: u32, floor: u8) {
        let food = ctx.snap.me.food;
        if food > floor || stock > 0 {
            self.no_food_told = None;
            return;
        }
        let emergency = food <= EMERGENCY_FOOD;
        if self.no_food_told == Some(emergency) {
            return;
        }
        self.no_food_told = Some(emergency);
        let ev = ctx
            .report(EventClass::Wake, "hungry_no_food", "")
            .magnitude(f32::from(20 - food.min(20)))
            .text(format!(
                "food {food}/20 and nothing to eat: hunt a cow, pig, sheep or chicken \
                 (collect beef 3, porkchop, mutton or chicken), eat it raw or smelt it"
            ));
        ctx.emit(ev);
    }
}
