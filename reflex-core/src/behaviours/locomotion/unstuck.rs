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
///
/// Once escalation starts, only net progress along the intended heading since the first
/// stuck tick ends it. A strafe that slides along the wall is motion but not progress; it
/// used to reset the counter, hand the legs back to the goal, and loop jump/strafe forever
/// without reaching the dig stage or waking the brain.
#[derive(Default)]
pub struct Unstuck {
    stuck: u32,
    intended: Option<Vec3>,
    anchor: Option<Vec3>,
    woke: bool,
    /// C6 owned the legs last tick, so the efference copy is its own command.
    driving: bool,
}

/// Net progress along the intended heading that counts as unstuck, in blocks.
const ESCAPED: f64 = 1.0;

impl Behaviour for Unstuck {
    fn id(&self) -> &'static str {
        "C6"
    }

    fn granted(&mut self, won: bool) {
        self.driving = won;
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let eff = ctx.pred.efference;
        let me = &ctx.snap.me;
        let commanded = eff.legs.filter(|l| expected_step(l).len() > MIN_PROGRESS);
        let moved = (me.pos - eff.from).horizontal().len();
        let escalating = self.stuck >= STUCK_TICKS;
        // While C6 drives, its own strafes keep `commanded` set; the episode ends when the
        // goal that wanted to move is gone.
        let abandoned = self.driving && ctx.knobs.goal.as_ref().and_then(|g| g.target).is_none();
        let progress = match (self.anchor, self.intended) {
            (Some(a), Some(h)) => (me.pos - a).horizontal().dot(h),
            _ => 0.0,
        };
        match commanded {
            Some(_) if escalating && progress < ESCAPED && !abandoned => self.stuck += 1,
            Some(l) if !escalating && moved < MIN_PROGRESS => {
                self.stuck += 1;
                self.intended
                    .get_or_insert(l.heading.horizontal().normalized());
                self.anchor.get_or_insert(me.pos);
            }
            _ => {
                *self = Self::default();
                return None;
            }
        }
        if self.stuck < STUCK_TICKS {
            return None;
        }
        if self.stuck >= WAKE_AFTER_TICKS && !self.woke {
            self.woke = true;
            ctx.emit(
                ctx.report(EventClass::Wake, "unstuck_failed", "C6")
                    .text("no progress after 3 escalation rounds"),
            );
        }
        let h = self.intended.unwrap_or(Vec3::new(0.0, 0.0, 1.0));
        let left = Vec3::new(h.z, 0.0, -h.x);
        // The dig stage is final: it holds until the way opens or the brain intervenes.
        let stage = ((self.stuck - STUCK_TICKS) / STAGE_TICKS).min(4);
        let cmds = match stage {
            0 => Commands::default().legs(LegsCmd::toward(h, Speed::Walk).jumping()),
            1 => Commands::default().legs(LegsCmd::toward(left, Speed::Walk)),
            2 => Commands::default().legs(LegsCmd::toward(-left, Speed::Walk)),
            3 => Commands::default().legs(LegsCmd::toward(-h, Speed::Walk)),
            _ => {
                let ahead = (me.pos + h).block();
                let target = if ctx.snap.blocks.get(ahead.above()).passable() {
                    ahead
                } else {
                    ahead.above()
                };
                let legs = LegsCmd::toward(h, Speed::Walk);
                if ctx.knobs.kept.contains(target) {
                    // The player's own block (a hut wall): never dug, wait for thought.
                    Commands::default().legs(legs)
                } else {
                    Commands::default()
                        .legs(legs)
                        .look_at(target.feet_centre() + Vec3::new(0.0, 0.5, 0.0))
                        .main(MainCmd::Dig(target))
                }
            }
        };
        let urgency = (self.stuck as f32 / WAKE_AFTER_TICKS as f32).min(1.0);
        Some(Bid::new(Priority::P4, urgency.max(0.3), "unstuck", cmds))
    }
}
