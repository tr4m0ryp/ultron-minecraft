use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::behaviours::common::{
    away_from, body_cells, escape_step, hitbox_cells, hitbox_cells_within,
};
use crate::channels::{Commands, LegsCmd, Speed};
use crate::snapshot::BlockKind;

/// R1: lava touches the body or is under the feet. Step to the nearest safe neighbour,
/// jumping while in lava. P0: lava deals 4 HP every half second.
///
/// Lava within `NEAR` blocks of the body (flowing lava spreads one block per 30 ticks, and a
/// player who just climbed out stands next to it) bids P1 to walk clear; a brain inhibition
/// can hold this tier when working next to lava on purpose.
#[derive(Default)]
pub struct LavaWithdrawal;

/// Horizontal clearance, in blocks, R1 keeps from lava at feet and head height.
pub const NEAR: f64 = 0.7;

impl Behaviour for LavaWithdrawal {
    fn id(&self) -> &'static str {
        "R1"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let me = &ctx.snap.me;
        let blocks = &ctx.snap.blocks;
        let body = hitbox_cells(me.pos);
        let under = body_cells(me.pos)[2];
        let in_lava_cell = body.iter().find(|p| blocks.get(**p) == BlockKind::Lava);
        let lava = in_lava_cell
            .copied()
            .or_else(|| (blocks.get(under) == BlockKind::Lava).then_some(under))?;
        let in_lava = in_lava_cell.is_some();
        let heading = match escape_step(blocks, me.pos, |_| true) {
            Some(p) => p.feet_centre() - me.pos,
            None => away_from(me.pos, lava.feet_centre()),
        };
        let mut legs = LegsCmd::toward(heading, Speed::Sprint);
        legs.jump = in_lava;
        Some(Bid::new(
            Priority::P0,
            1.0,
            "lava_withdraw",
            Commands::default().legs(legs),
        ))
    }
}

/// Lava close to but not touching the body: walk (never sprint-jump) to a cell clear of it.
fn near(ctx: &Ctx) -> Option<Bid> {
    let me = &ctx.snap.me;
    let blocks = &ctx.snap.blocks;
    let lava = hitbox_cells_within(me.pos, NEAR)
        .into_iter()
        .find(|p| blocks.get(*p) == BlockKind::Lava)?;
    let heading = match escape_step(blocks, me.pos, |_| true) {
        Some(p) => p.feet_centre() - me.pos,
        None => away_from(me.pos, lava.feet_centre()),
    };
    Some(Bid::new(
        Priority::P1,
        0.8,
        "lava_near",
        Commands::default().legs(LegsCmd::toward(heading, Speed::Walk)),
    ))
}
