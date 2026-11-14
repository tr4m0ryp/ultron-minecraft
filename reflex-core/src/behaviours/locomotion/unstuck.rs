use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::channels::{Commands, LegsCmd, MainCmd, Speed};
use crate::events::EventClass;
use crate::math::Vec3;
use crate::predict::expected_step;

/// Observed displacement below this while commanding motion counts as stuck.
const MIN_PROGRESS: f64 = 0.05;
pub const STUCK_TICKS: u32 = 20;
const STAGE_TICKS: u32 = 15;
/// Three escalation rounds, about 200 ticks after the first stuck tick, then wake the brain.
pub const WAKE_AFTER_TICKS: u32 = 200;

/// C6: intended motion (efference copy) but observed displacement under 0.05 blocks per
/// tick for 20 ticks. Escalate: jump, strafe left, strafe right, back up, mine the block
/// ahead at feet and head height; after about 200 ticks wake the brain.
#[derive(Default)]
pub struct Unstuck {
    stuck: u32,
    intended: Option<Vec3>,
    woke: bool,
}

impl Behaviour for Unstuck {
    fn id(&self) -> &'static str {
        "C6"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let eff = ctx.pred.efference;
        let me = &ctx.snap.me;
        let commanded = eff.legs.filter(|l| expected_step(l).len() > MIN_PROGRESS);
        let moved = (me.pos - eff.from).horizontal().len();
        match commanded {
            Some(l) if moved < MIN_PROGRESS => {
                self.stuck += 1;
                self.intended.get_or_insert(l.heading.horizontal().normalized());
            }
            _ => {
                self.stuck = 0;
                self.intended = None;
                self.woke = false;
                return None;
            }
        }
        if self.stuck < STUCK_TICKS {
            return None;
        }
        if self.stuck >= WAKE_AFTER_TICKS && !self.woke {
            self.woke = true;
            ctx.emit(ctx.report(EventClass::Wake, "unstuck_failed", "C6").text("no progress after 3 escalation rounds"));
        }
        let h = self.intended.unwrap_or(Vec3::new(0.0, 0.0, 1.0));
        let left = Vec3::new(h.z, 0.0, -h.x);
        let stage = ((self.stuck - STUCK_TICKS) / STAGE_TICKS) % 5;
        let cmds = match stage {
            0 => Commands::default().legs(LegsCmd::toward(h, Speed::Walk).jumping()),
            1 => Commands::default().legs(LegsCmd::toward(left, Speed::Walk)),
            2 => Commands::default().legs(LegsCmd::toward(-left, Speed::Walk)),
            3 => Commands::default().legs(LegsCmd::toward(-h, Speed::Walk)),
            _ => {
                let ahead = (me.pos + h).block();
                let target = if ctx.snap.blocks.get(ahead.above()).passable() { ahead } else { ahead.above() };
                Commands::default()
                    .legs(LegsCmd::toward(h, Speed::Walk))
                    .look_at(target.feet_centre() + Vec3::new(0.0, 0.5, 0.0))
                    .main(MainCmd::Dig(target))
            }
        };
        let urgency = (self.stuck as f32 / WAKE_AFTER_TICKS as f32).min(1.0);
        Some(Bid::new(Priority::P4, urgency.max(0.3), "unstuck", cmds))
    }
}
