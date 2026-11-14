use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::behaviours::common::heading_to;
use crate::channels::{Commands, GazeCmd, LegsCmd, MainCmd, Speed};
use crate::math::BlockPos;
use crate::snapshot::{BlockKind, LocalBlocks};

/// R8: head under water with air below air_floor. Hold jump toward the nearest column open
/// to air; if none, mine upward. P1, P0 at 20 ticks of air or less.
#[derive(Default)]
pub struct SurfaceForAir;

const SCAN_UP: i32 = 16;
const CRITICAL_AIR: i32 = 20;

/// The column above `p` reaches air through water only.
fn open_column(blocks: &LocalBlocks, p: BlockPos) -> bool {
    for dy in 0..SCAN_UP {
        match blocks.get(p.offset(0, dy, 0)) {
            BlockKind::Air => return true,
            BlockKind::Water | BlockKind::Climbable => continue,
            _ => return false,
        }
    }
    false
}

impl Behaviour for SurfaceForAir {
    fn id(&self) -> &'static str {
        "R8"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let me = &ctx.snap.me;
        let floor = ctx.knobs.gains.air_floor;
        if !me.head_in_water || me.water_breathing || me.air >= floor {
            return None;
        }
        let priority = if me.air <= CRITICAL_AIR { Priority::P0 } else { Priority::P1 };
        let urgency = 1.0 - me.air.max(0) as f32 / floor.max(1) as f32;
        let head = me.pos.block().above();
        let up = GazeCmd::Angles { yaw: me.yaw, pitch: -90.0 };
        if open_column(&ctx.snap.blocks, head) {
            let cmds = Commands::default().legs(LegsCmd::stop().jumping()).gaze(up);
            return Some(Bid::new(priority, urgency, "surface", cmds));
        }
        let side = (1..=4).find_map(|r| {
            BlockPos::HORIZONTAL
                .iter()
                .map(|&(dx, dz)| head.offset(dx * r, 0, dz * r))
                .find(|p| open_column(&ctx.snap.blocks, *p))
        });
        let cmds = match side {
            Some(p) => Commands::default()
                .legs(LegsCmd::toward(heading_to(me.pos, p.feet_centre()), Speed::Sprint).jumping()),
            None => Commands::default().gaze(up).main(MainCmd::Dig(head.above())),
        };
        Some(Bid::new(priority, urgency, "surface", cmds))
    }
}
