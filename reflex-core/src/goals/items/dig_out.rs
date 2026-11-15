//! Digging one chosen block out of the way: walk within reach, hold the right tool, break
//! it. Shared by a placement clearing its cell and by the explicit break goal.

use super::equip;
use crate::channels::{Commands, LegsCmd, MainCmd};
use crate::goals::ports::{NavTarget, ToolNeed};
use crate::goals::task::{GoalCx, Plan};
use crate::goals::travel::{Walk, Walker};
use crate::math::{BlockPos, Vec3};

/// Reach for digging a block from where the body stands.
const REACH: f64 = 4.0;
/// Ticks one block may take to dig out (falling sand or gravel refills a cell) before the
/// caller gives up on it.
pub const DIG_OUT_LIMIT: u32 = 400;

/// One tick of digging `pos`, which holds `held`. `ticks` counts the ticks spent (not
/// pre-empted). `Err` is the reason the block cannot be dug out.
pub fn dig_out(
    cx: &mut GoalCx,
    walker: &mut Walker,
    ticks: &mut u32,
    pos: BlockPos,
    held: &str,
    note: &'static str,
) -> Result<Plan, String> {
    let at = format!("{} {} {}", pos.x, pos.y, pos.z);
    if *ticks > DIG_OUT_LIMIT {
        return Err(format!("could not dig {held} out at {at}"));
    }
    match cx.world.tool_for(held, cx.snap()) {
        ToolNeed::Missing(t) => return Err(format!("missing tool for {held} at {at}: needs {t}")),
        ToolNeed::Have(slot) => {
            if let Some(c) = equip(&cx.snap().me, slot) {
                return Ok(Plan::new("goal_equip", c));
            }
        }
        ToolNeed::Hand => {}
    }
    let centre = pos.feet_centre() + Vec3::new(0.0, 0.5, 0.0);
    if cx.snap().me.eye().dist(centre) > REACH {
        let t = NavTarget::Block {
            pos,
            radius: REACH - 1.0,
        };
        return match walker.walk(cx, t, note) {
            Walk::Moving(p) => Ok(p),
            Walk::Arrived => Ok(Plan::hold(note)),
            Walk::Failed(f) => Err(format!("cannot get within reach of {at} ({f:?})")),
        };
    }
    walker.stop(cx);
    if !cx.preempted {
        *ticks += 1;
    }
    // Digging a cell on purpose: the ledger lets go of a block the player placed there.
    cx.ledger.forget(pos);
    let c = Commands::default()
        .legs(LegsCmd::stop())
        .look_at(centre)
        .main(MainCmd::Dig(pos));
    Ok(Plan::new(note, c))
}
