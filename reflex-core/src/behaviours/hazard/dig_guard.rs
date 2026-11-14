use crate::behaviour::{Behaviour, Bid, Ctx, Priority, Veto};
use crate::channels::{Commands, MainCmd};
use crate::events::EventClass;
use crate::snapshot::BlockKind;

/// R10: refuses a commanded dig of the block under the feet when air, liquid, unknown or
/// void lies within 3 blocks beneath it, or of the block above the head when a gravity
/// block sits on it. P0 when lava or void is below (the floor).
#[derive(Default)]
pub struct DigGuard {
    reported_goal: Option<Option<u64>>,
}

impl Behaviour for DigGuard {
    fn id(&self) -> &'static str {
        "R10"
    }

    fn evaluate(&mut self, _ctx: &Ctx) -> Option<Bid> {
        None
    }

    fn veto(&mut self, ctx: &Ctx, plan: &mut Commands) -> Option<Veto> {
        let Some(MainCmd::Dig(target)) = plan.main else { return None };
        let feet = ctx.snap.me.pos.block();
        let blocks = &ctx.snap.blocks;
        let mut lethal = false;
        let unsafe_dig = if target == feet.below() {
            (1..=3).any(|d| {
                let p = target.offset(0, -d, 0);
                let k = blocks.get(p);
                let void = p.y < ctx.snap.world_min_y;
                lethal |= k == BlockKind::Lava || void;
                void || matches!(k, BlockKind::Air | BlockKind::Unknown) || k.is_liquid()
            })
        } else if target == feet.above().above() {
            blocks.get(target.above()) == BlockKind::Gravity
        } else {
            false
        };
        if !unsafe_dig {
            return None;
        }
        plan.main = None;
        let goal = ctx.knobs.goal.as_ref().map(|g| g.id);
        if self.reported_goal != Some(goal) {
            self.reported_goal = Some(goal);
            ctx.emit(ctx.report(EventClass::Digest, "dig_vetoed", "R10"));
        }
        let priority = if lethal { Priority::P0 } else { Priority::P1 };
        Some(Veto { priority, note: "dig_guard" })
    }
}
