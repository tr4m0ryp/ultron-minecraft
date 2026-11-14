use crate::behaviour::{Ctx, Priority};
use crate::events::{Event, EventClass};
use crate::knobs::Knobs;

/// A brain inhibition was ignored because predicted harm is lethal; the brain must know.
pub fn override_event(ctx: &Ctx, id: &str, note: &str) -> Event {
    ctx.report(EventClass::Wake, "override", id)
        .critical()
        .text(format!(
            "{id} {note} fired despite inhibition: predicted harm lethal"
        ))
}

pub fn suppressed(ctx: &Ctx, id: &str, note: &str) -> Event {
    ctx.report(EventClass::Log, "suppressed", id)
        .text(note.to_string())
}

/// Onset of a reflex firing. P0 is always reported and wakes the brain when hp is under
/// the floor (near-death).
pub fn fired(ctx: &Ctx, id: &str, note: &str, p: Priority, knobs: &Knobs) -> Event {
    let near_death = p == Priority::P0 && ctx.snap.me.health < knobs.gains.hp_floor;
    let class = match p {
        _ if near_death => EventClass::Wake,
        Priority::P5 | Priority::P6 => EventClass::Log,
        _ => EventClass::Digest,
    };
    let mut e = ctx.report(class, note, "");
    if p == Priority::P0 {
        e = e.critical();
    }
    e.text(format!("{id} {note}"))
}

pub fn armed_fired(ctx: &Ctx, arm_id: &str, note: &str) -> Event {
    ctx.report(EventClass::Wake, "armed_fired", note)
        .text(format!("armed response {arm_id} fired: {note}"))
}

pub fn goal_interrupted(ctx: &Ctx, holder: &str) -> Event {
    let spec = ctx
        .knobs
        .goal
        .as_ref()
        .map(|g| g.spec.as_str())
        .unwrap_or("");
    ctx.report(EventClass::Wake, "goal_interrupted", holder)
        .text(format!(
            "goal \"{spec}\" has lost the legs to {holder} for 200 ticks"
        ))
}
