//! Maps the open goal vocabulary onto what reflex-core can execute today. Supporting a new
//! goal is one match arm here plus its name in `SUPPORTED_GOALS`.

use reflex_core::knobs::{GainName, Knobs};
use reflex_core::Snapshot;

use crate::wire::{Ack, GoalName, GoalParams};

pub const SUPPORTED_GOALS: [GoalName; 5] = [
    GoalName::End,
    GoalName::Follow,
    GoalName::GoTo,
    GoalName::ComeHere,
    GoalName::Stay,
];

const FOLLOW_DEFAULT: f64 = 3.0;

pub fn set_goal(
    knobs: &mut Knobs,
    snap: &Snapshot,
    goal: GoalName,
    params: &GoalParams,
    priority: Option<f32>,
) -> Result<Ack, String> {
    params.check(goal)?;
    let priority = priority.unwrap_or(0.5);
    if !priority.is_finite() {
        return Err("priority must be a number in 0..1".into());
    }
    let spec = params.spec(goal);
    let me = snap.me.pos;
    let (target, solo) = match goal {
        GoalName::End => {
            let ended = knobs.end_goal();
            let next = knobs.goal.as_ref().map_or("none", |g| g.spec.as_str());
            return Ok(Ack {
                id: ended.as_ref().map(|g| g.id),
                done: match ended {
                    Some(g) => format!("ended #{} {}; active now: {next}", g.id, g.spec),
                    None => "no active goal".into(),
                },
            });
        }
        GoalName::GoTo => {
            let [x, y, z] = params.pos.unwrap_or_default();
            (Some(reflex_core::Vec3::new(x, y, z)), true)
        }
        GoalName::ComeHere => (Some(companion(snap, params)?.pos), false),
        GoalName::Stay => (Some(me.block().feet_centre()), true),
        GoalName::Follow => {
            companion(snap, params)?;
            let d = params.distance.unwrap_or(FOLLOW_DEFAULT).clamp(1.0, 32.0);
            knobs.set_gain(GainName::Leash, d as f32);
            (None, false)
        }
        other => {
            let names: Vec<&str> = SUPPORTED_GOALS.iter().map(|g| g.label()).collect();
            return Err(format!(
                "goal {} is not supported by this executor yet; supported now: {}",
                other.label(),
                names.join(", ")
            ));
        }
    };
    let id = knobs.set_goal(spec.clone(), priority, target, solo);
    let done = match target {
        Some(t) if goal != GoalName::GoTo => {
            format!("goal #{id} {spec} to {:.1},{:.1},{:.1}", t.x, t.y, t.z)
        }
        _ => format!("goal #{id} {spec}"),
    };
    Ok(Ack { id: Some(id), done })
}

fn companion<'a>(
    snap: &'a Snapshot,
    params: &GoalParams,
) -> Result<&'a reflex_core::snapshot::Companion, String> {
    let c = snap
        .companion
        .as_ref()
        .ok_or("companion not visible right now")?;
    match &params.companion {
        Some(name) if !name.eq_ignore_ascii_case(&c.name) => Err(format!(
            "only the companion {} can be tracked, not {name}",
            c.name
        )),
        _ => Ok(c),
    }
}
