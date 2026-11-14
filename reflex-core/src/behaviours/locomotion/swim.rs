use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::behaviours::common::{heading_to, nearest_shore};
use crate::channels::{Commands, LegsCmd, Speed};

/// Shores beyond the sensing box are not trusted.
const SHORE_RADIUS: i32 = 6;

/// C3 swim (tread and land): in water and not standing (feet off the bottom, or the head
/// under). Hold jump so the head stays out instead of letting go once it breaks the surface
/// (R8 only acts below air_floor, which made the bot sink and resurface in a loop), and
/// stroke toward the nearest shore in view; holding jump against the bank climbs out. A
/// goal with a target swims itself (the goal executor holds jump in water), so C3 stands
/// down for it.
#[derive(Default)]
pub struct Swim;

impl Behaviour for Swim {
    fn id(&self) -> &'static str {
        "C3"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let me = &ctx.snap.me;
        if !me.in_water || (me.on_ground && !me.head_in_water) {
            return None;
        }
        if ctx.knobs.goal.as_ref().is_some_and(|g| g.target.is_some()) {
            return None;
        }
        let (legs, note) = match nearest_shore(&ctx.snap.blocks, me.pos, SHORE_RADIUS) {
            Some(p) => (
                LegsCmd::toward(heading_to(me.pos, p.feet_centre()), Speed::Walk).jumping(),
                "swim_to_shore",
            ),
            None => (LegsCmd::stop().jumping(), "tread_water"),
        };
        Some(Bid::new(
            Priority::P4,
            0.6,
            note,
            Commands::default().legs(legs),
        ))
    }
}
