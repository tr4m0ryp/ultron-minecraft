use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::channels::Commands;
use crate::events::EventClass;
use crate::math::Vec3;

/// O1: chat from the companion at any distance, from another player within 16 blocks, or
/// any chat naming the bot. Turn the head to the speaker and hold for 40 ticks unless
/// something more salient takes the gaze. Wakes the brain with the line. Live, the
/// companion asked "can you come over here?" from 22 blocks without the bot's name, and
/// the line never reached thought: no reply, no goal.
#[derive(Default)]
pub struct LookAtSpeaker {
    target: Option<(Vec3, u64)>,
}

const HOLD: u64 = 40;
const RANGE: f64 = 16.0;

impl Behaviour for LookAtSpeaker {
    fn id(&self) -> &'static str {
        "O1"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let snap = ctx.snap;
        let name = ctx.bot_name.to_lowercase();
        for chat in &snap.chat {
            if chat.sender.eq_ignore_ascii_case(ctx.bot_name) {
                continue;
            }
            let pos = chat.sender_pos.or_else(|| {
                snap.companion
                    .as_ref()
                    .filter(|c| c.name == chat.sender)
                    .map(|c| c.pos)
            });
            let near = pos.is_some_and(|p| p.dist(snap.me.pos) <= RANGE);
            let named = !name.is_empty() && chat.text.to_lowercase().contains(&name);
            // By the configured name: the companion's entity is not tracked beyond the
            // server's entity range, but their chat still arrives from any distance.
            let companion = snap
                .companion_name
                .as_deref()
                .or(snap.companion.as_ref().map(|c| c.name.as_str()))
                .is_some_and(|n| n.eq_ignore_ascii_case(&chat.sender));
            if !(near || named || companion) {
                continue;
            }
            // Critical: a human line skips the 5 s wake gap and the repeat filter, so a
            // reply is never held behind a goal event and "come here" twice wakes twice.
            ctx.emit(
                ctx.report(EventClass::Wake, "chat", &chat.sender)
                    .text(format!("<{}> {}", chat.sender, chat.text))
                    .critical(),
            );
            if let Some(p) = pos {
                self.target = Some((p + Vec3::new(0.0, 1.62, 0.0), snap.tick + HOLD));
            }
        }
        let (p, until) = self.target?;
        if snap.tick >= until {
            self.target = None;
            return None;
        }
        Some(Bid::new(
            Priority::P4,
            0.8,
            "look_at_speaker",
            Commands::default().look_at(p),
        ))
    }
}
