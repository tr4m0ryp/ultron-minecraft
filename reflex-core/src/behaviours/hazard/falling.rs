use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::behaviours::common::escape_step;
use crate::channels::{Commands, LegsCmd, MainCmd, Speed};
use crate::snapshot::EntityKind;

/// R9: head inside a solid block (suffocating, P0), or a falling block entity in our
/// column within 3 blocks above (P1). Step horizontally out of the column; if enclosed
/// while suffocating, mine the block at head height.
#[derive(Default)]
pub struct FallingBlocks;

impl Behaviour for FallingBlocks {
    fn id(&self) -> &'static str {
        "R9"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let me = &ctx.snap.me;
        let feet = me.pos.block();
        let head = feet.above();
        let head_kind = ctx.snap.blocks.get(head);
        let suffocating = head_kind.supports() && !head_kind.passable();
        let falling = ctx.snap.entities.iter().any(|e| {
            e.kind == EntityKind::FallingBlock
                && (e.pos.x - me.pos.x).abs() < 0.9
                && (e.pos.z - me.pos.z).abs() < 0.9
                && e.pos.y > me.pos.y
                && e.pos.y - (me.pos.y + 1.8) <= 3.0
        });
        if !suffocating && !falling {
            return None;
        }
        let out = escape_step(&ctx.snap.blocks, me.pos, |p| p.x != feet.x || p.z != feet.z);
        let priority = if suffocating {
            Priority::P0
        } else {
            Priority::P1
        };
        let cmds = match out {
            Some(p) => {
                Commands::default().legs(LegsCmd::toward(p.feet_centre() - me.pos, Speed::Sprint))
            }
            None if suffocating => Commands::default()
                .look_at(head.feet_centre())
                .main(MainCmd::Dig(head)),
            None => return None,
        };
        let note = if suffocating {
            "suffocation_escape"
        } else {
            "falling_block_dodge"
        };
        Some(Bid::new(priority, 1.0, note, cmds))
    }
}
