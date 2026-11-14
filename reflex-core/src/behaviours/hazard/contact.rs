use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::behaviours::common::escape_step;
use crate::channels::{Commands, LegsCmd, Speed};
use crate::snapshot::BlockKind;

/// R4: the body touches fire, campfire, cactus, berry bush, wither rose, powder snow, or
/// stands on magma without sneaking. Step to the nearest safe neighbour; on magma with no
/// way off, sneak.
#[derive(Default)]
pub struct ContactHazard;

impl Behaviour for ContactHazard {
    fn id(&self) -> &'static str {
        "R4"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let me = &ctx.snap.me;
        let blocks = &ctx.snap.blocks;
        let feet = me.pos.block();
        let touching = hitbox_cells(me.pos)
            .into_iter()
            .map(|p| blocks.get(p))
            .any(|k| k.contact_hazard() && k != BlockKind::Lava);
        let side_cactus = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dz)| {
            let p = feet.offset(dx, 0, dz);
            blocks.get(p) == BlockKind::Cactus
                && (p.feet_centre() - me.pos).horizontal().len() < 0.85
        });
        let magma = blocks.get(feet.below()) == BlockKind::Magma && !me.sneaking;
        if !(touching || side_cactus || magma) {
            return None;
        }
        let cmds = match escape_step(blocks, me.pos, |p| p != feet) {
            Some(p) => {
                Commands::default().legs(LegsCmd::toward(p.feet_centre() - me.pos, Speed::Walk))
            }
            None if magma => Commands::default().legs(LegsCmd {
                speed: Speed::Sneak,
                ..LegsCmd::stop()
            }),
            None => return None,
        };
        Some(Bid::new(Priority::P1, 0.9, "contact_hazard", cmds))
    }
}
