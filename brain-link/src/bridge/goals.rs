//! Maps the open goal vocabulary onto the goal layer's spec strings (`mine stone 3`,
//! `give companion bread 2`, `place cobblestone 10 64 -3`), validated with the goal layer's
//! own parser before the goal is set, so a bad goal fails here and not a tick later.

use reflex_core::goals;
use reflex_core::knobs::{GainName, Knobs};
use reflex_core::{Snapshot, Vec3};

use crate::wire::{Ack, GoalName, GoalParams};

/// Every goal name the executor runs.
pub const SUPPORTED_GOALS: [GoalName; 14] = GoalName::ALL;

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
    if goal == GoalName::End {
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
    let (spec, target, solo) = layer_spec(snap, goal, params)?;
    goals::parse(&spec, target)?;
    if goal == GoalName::Follow {
        let d = params.distance.unwrap_or(FOLLOW_DEFAULT).clamp(1.0, 32.0);
        knobs.set_gain(GainName::Leash, d as f32);
    }
    let id = knobs.set_goal(spec.clone(), priority, target, solo);
    let done = match target {
        Some(t) if matches!(goal, GoalName::ComeHere | GoalName::Stay) => {
            format!("goal #{id} {spec} to {:.1},{:.1},{:.1}", t.x, t.y, t.z)
        }
        _ => format!("goal #{id} {spec}"),
    };
    Ok(Ack { id: Some(id), done })
}

/// The goal layer spec, the knob target, and whether the goal suspends the companion leash.
pub fn layer_spec(
    snap: &Snapshot,
    goal: GoalName,
    p: &GoalParams,
) -> Result<(String, Option<Vec3>, bool), String> {
    let kind = || p.kind.as_deref().map(str::trim).unwrap_or_default();
    let count = p.count.unwrap_or(1);
    let pos = p.pos.map(|[x, y, z]| Vec3::new(x, y, z));
    let xyz = |v: Vec3| format!("{} {} {}", v.x, v.y, v.z);
    let who = person(snap, p);
    Ok(match goal {
        GoalName::End => ("end".into(), None, false),
        GoalName::Follow => {
            let d = p.distance.unwrap_or(FOLLOW_DEFAULT).clamp(1.0, 32.0);
            (format!("follow {who} {d}"), None, false)
        }
        GoalName::GoTo => {
            let v = pos.ok_or("goal go_to needs param 'pos'")?;
            (format!("go_to {}", xyz(v)), Some(v), true)
        }
        GoalName::ComeHere => {
            let at = snap.companion.as_ref().map(|c| c.pos);
            ("come_here".into(), at, false)
        }
        GoalName::Stay => ("stay".into(), Some(snap.me.pos.block().feet_centre()), true),
        GoalName::Mine => (format!("mine {} {count}", kind()), None, true),
        GoalName::Collect => (format!("collect {} {count}", kind()), None, true),
        GoalName::Attack => (format!("attack {}", kind()), None, true),
        GoalName::Guard => (format!("guard {who}"), None, false),
        GoalName::Craft => (format!("craft {} {count}", kind()), None, true),
        GoalName::Give => (format!("give {who} {} {count}", kind()), None, false),
        GoalName::Place => {
            let v = pos.ok_or("goal place needs param 'pos'")?;
            let to =
                p.to.map(|[x, y, z]| format!(" {}", xyz(Vec3::new(x, y, z))));
            let spec = format!("place {} {}{}", kind(), xyz(v), to.unwrap_or_default());
            (spec, Some(v), true)
        }
        GoalName::SleepInBed => ("sleep_in_bed".into(), None, true),
        GoalName::EatNow => ("eat_now".into(), None, false),
    })
}

/// `companion` for the configured companion, else the named player.
fn person(snap: &Snapshot, p: &GoalParams) -> String {
    match p.companion.as_deref().map(str::trim) {
        None | Some("") => "companion".into(),
        Some(n)
            if snap
                .companion
                .as_ref()
                .is_some_and(|c| c.name.eq_ignore_ascii_case(n)) =>
        {
            "companion".into()
        }
        Some(n) => n.split_whitespace().next().unwrap_or("companion").into(),
    }
}
