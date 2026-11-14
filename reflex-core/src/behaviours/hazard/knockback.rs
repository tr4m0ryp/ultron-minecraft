use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::behaviours::common::{heading_to, nearest_ledge, unsafe_drop};
use crate::channels::{Commands, GazeCmd, LegsCmd, Speed};
use crate::math::{yaw_pitch_to, Vec3};

/// Ledges farther than this are out of air-strafe range (air control adds only about 0.026
/// blocks/tick per tick of input).
const LEDGE_RADIUS: i32 = 3;

/// R15 knockback recovery: airborne (not by our own choice: R5 never walks off an unsafe
/// drop, so an airborne state over one means a push) with a harmful landing predicted and a
/// ledge still at or below the feet within reach. Air-strafe back toward the ledge with
/// sprint held and the yaw turned toward it, which is the most air control vanilla allows.
/// P0 when the landing would be lethal, else P1. The landing save itself (block or water)
/// is R6's.
///
/// On the ground with the centre already past the edge (a hit pushed us there; R5 stops
/// our own steps well short of it), step back to the middle of the nearest standable cell
/// at P1 before the next hit finishes the job. Live, a zombie's push moves the player only
/// about 0.45 blocks, so the knock-off usually takes two or three hits and this step is
/// where it is won.
#[derive(Default)]
pub struct KnockbackRecovery;

impl Behaviour for KnockbackRecovery {
    fn id(&self) -> &'static str {
        "R15"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let me = &ctx.snap.me;
        if me.in_water {
            return None;
        }
        if me.on_ground {
            if !unsafe_drop(&ctx.snap.blocks, me.pos.block()) {
                return None;
            }
            let ledge = nearest_ledge(&ctx.snap.blocks, me.pos, 1)?;
            let back = heading_to(me.pos, ledge.cell.feet_centre());
            let cmds = Commands::default().legs(LegsCmd::toward(back, Speed::Walk));
            return Some(Bid::new(Priority::P1, 1.0, "edge_step_back", cmds));
        }
        let fall = ctx.pred.fall?;
        if !fall.into_void && fall.damage < 1.0 {
            return None;
        }
        let ledge = nearest_ledge(&ctx.snap.blocks, me.pos, LEDGE_RADIUS)?;
        let lethal = fall.into_void || fall.damage >= me.hp_total();
        let priority = if lethal { Priority::P0 } else { Priority::P1 };
        let legs = LegsCmd::toward(ledge.heading, Speed::Sprint);
        let (yaw, _) = yaw_pitch_to(Vec3::ZERO, ledge.heading);
        let full = Commands::default().legs(legs).gaze(GazeCmd::Angles {
            yaw,
            pitch: me.pitch,
        });
        let degraded = Commands::default().legs(legs);
        Some(Bid::new(priority, 1.0, "knockback_recover", full).degraded(degraded))
    }
}
