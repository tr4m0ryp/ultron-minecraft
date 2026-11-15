//! Turning a path's hand work (dig a block in the way, place a scaffold block) into
//! commands, with the right tool or block in hand first.

use crate::behaviours::common::heading_to;
use crate::channels::{Commands, LegsCmd, MainCmd, Speed};
use crate::goals::items::equip;
use crate::goals::ports::{NavFailure, Steer, ToolNeed, Work};
use crate::goals::task::{GoalCx, Plan};
use crate::goals::travel::face_point;
use crate::goals::travel::route::SCAFFOLD;
use crate::math::Vec3;
use crate::snapshot::Slot;

/// Close enough to the hold point to stand still while working.
const HOLD: f64 = 0.2;

/// The scaffold block to spend, cheapest first.
pub fn scaffold_slot(cx: &GoalCx) -> Option<Slot> {
    SCAFFOLD.iter().find_map(|b| cx.world.item_slot(b))
}

pub fn work_plan(
    cx: &mut GoalCx,
    s: Steer,
    w: Work,
    note: &'static str,
) -> Result<Plan, NavFailure> {
    let me = cx.snap().me.clone();
    let off = (me.pos - s.waypoint).horizontal().len();
    let mut legs = if off > HOLD || s.sneak {
        LegsCmd::toward(heading_to(me.pos, s.waypoint), Speed::Sneak)
    } else {
        LegsCmd::stop()
    };
    legs.jump = s.jump;
    let base = Commands::default().legs(legs);
    match w {
        Work::Dig(c) => {
            // The path was planned before this block was placed or seen placed.
            if cx.ledger.blocks_dig(c) {
                return Err(NavFailure::Unreachable);
            }
            let tool = cx
                .world
                .block_at(c)
                .map(|b| cx.world.tool_for(&b, cx.snap()));
            if let Some(ToolNeed::Have(slot)) = tool {
                if let Some(e) = equip(&me, slot) {
                    return Ok(Plan::new("goal_equip", e));
                }
            }
            let centre = c.feet_centre() + Vec3::new(0.0, 0.5, 0.0);
            Ok(Plan::new(note, base.look_at(centre).main(MainCmd::Dig(c))))
        }
        Work::Place { at, click } => {
            let Some(slot) = scaffold_slot(cx) else {
                return Err(NavFailure::NoScaffold);
            };
            if let Some(mut e) = equip(&me, slot) {
                e.legs = Some(legs);
                return Ok(Plan::new("goal_equip", e));
            }
            let aim = base.look_at(face_point(at));
            let main = if click {
                MainCmd::PlaceAt {
                    cell: at.cell,
                    against: at.against,
                }
            } else {
                MainCmd::SelectSlot(me.selected_slot)
            };
            Ok(Plan::new(note, aim.main(main)))
        }
    }
}
