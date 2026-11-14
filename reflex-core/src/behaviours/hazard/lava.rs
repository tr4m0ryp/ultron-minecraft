use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::behaviours::common::{away_from, body_cells, escape_step, hitbox_cells};
use crate::channels::{Commands, LegsCmd, Speed};
use crate::snapshot::BlockKind;

/// R1: lava touches the body or is under the feet. Step to the nearest safe neighbour,
/// jumping while in lava. P0: lava deals 4 HP every half second.
#[derive(Default)]
pub struct LavaWithdrawal;

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
