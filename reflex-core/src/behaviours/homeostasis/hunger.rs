use super::eat::{
    best_food, best_ready_food, eat_commands, food_servings, meal_is_safe, raw_servings,
    EATING_TICKS,
};
use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::channels::MainCmd;
use crate::events::EventClass;

/// H1: food at or below hunger_floor with no hostile within 12 blocks: eat the best food
/// that is ready to eat. Raw meat, fish and potatoes are kept for the furnace until the
/// emergency at food 6 or less (sprinting impossible, so fleeing is too): P1, eats anything
/// unless a hostile is within 4. Never starts a meal in an archer's line of fire
/// (`meal_is_safe`). Live, the reflex ate the raw beef a `smelt cooked_beef 3` was about to
/// cook, and the smelt failed one short.
#[derive(Default)]
pub struct Hunger {
    low_stock_reported: bool,
    /// Hungry with nothing to eat was told to thought; `Some(true)` once it was an emergency.
    no_food_told: Option<bool>,
    /// Hungry with only raw food: this spell of it was reported.
    raw_told: bool,
    /// Tick of the last time that woke thought.
    raw_woke: Option<u64>,
}

const EMERGENCY_FOOD: u8 = 6;
const LOW_STOCK: u32 = 5;
/// Ticks (5 minutes) before "only raw food" may wake thought again; sooner than that a new
/// spell goes into the digest. Each raw meal at the emergency level lifts the food bar
/// back over it with raw food still the only food, and every one of those used to be a
/// fresh wake: one per meal.
pub const RAW_RETELL_TICKS: u64 = 6000;

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
        self.tell_raw_only(ctx, floor);
        if me.food > floor {
            return None;
        }
        let emergency = me.food <= EMERGENCY_FOOD;
        let safe_radius = if emergency { 4.0 } else { 12.0 };
        if ctx.snap.nearest_hostile(safe_radius).is_some() || !meal_is_safe(ctx.snap) {
            return None;
        }
        let slot = if emergency {
            best_food(me, false)?
        } else {
            best_ready_food(me)?
        };
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

    /// Hungry and the only food is raw: the reflex leaves it for the furnace, so thought
    /// hears that a smelt (or an explicit meal) is up to it. One spell lasts until the body
    /// is fed, the raw food is gone or ready food turns up; the emergency meals in between
    /// do not end it. It wakes thought once per `RAW_RETELL_TICKS`; a new spell before that
    /// is a digest line.
    fn tell_raw_only(&mut self, ctx: &Ctx, floor: u8) {
        let me = &ctx.snap.me;
        let raw = raw_servings(me);
        if me.food > floor || raw == 0 || best_ready_food(me).is_some() {
            self.raw_told = false;
            return;
        }
        if self.raw_told || me.food <= EMERGENCY_FOOD {
            return;
        }
        self.raw_told = true;
        let now = ctx.snap.tick;
        let quiet = self.raw_woke.is_some_and(|t| now < t + RAW_RETELL_TICKS);
        let class = if quiet {
            EventClass::Digest
        } else {
            self.raw_woke = Some(now);
            EventClass::Wake
        };
        let ev = ctx
            .report(class, "hungry_raw_food", "")
            .magnitude(f32::from(20 - me.food.min(20)))
            .text(format!(
                "food {}/20 and only raw food ({raw}); kept for the furnace: smelt it \
                 (smelt cooked_beef {raw}, cooked_porkchop, cooked_mutton or cooked_chicken), \
                 or eat_now to eat it raw; the body eats it raw at food {EMERGENCY_FOOD}",
                me.food
            ));
        ctx.emit(ev);
    }
}
