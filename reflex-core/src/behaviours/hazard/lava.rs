use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::behaviours::common::{away_from, body_cells, escape_step};
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
        let cells = body_cells(me.pos);
        let lava = cells.iter().find(|p| blocks.get(**p) == BlockKind::Lava)?;
        let in_lava = blocks.get(cells[0]) == BlockKind::Lava || blocks.get(cells[1]) == BlockKind::Lava;
        let heading = match escape_step(blocks, me.pos, |_| true) {
            Some(p) => p.feet_centre() - me.pos,
            None => away_from(me.pos, lava.feet_centre()),
        };
        let mut legs = LegsCmd::toward(heading, Speed::Sprint);
        legs.jump = in_lava;
        Some(Bid::new(Priority::P0, 1.0, "lava_withdraw", Commands::default().legs(legs)))
    }
}
