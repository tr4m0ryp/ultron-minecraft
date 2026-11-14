//! Converts request arguments that need the executor's state (position, goal, catalog).

use std::sync::OnceLock;

use reflex_core::behaviours::default_catalog;
use reflex_core::knobs::Scope;
use reflex_core::math::{Aabb, Vec3};
use reflex_core::Snapshot;

use crate::host::Host;
use crate::wire::WireScope;

const TICKS_PER_S: f64 = 20.0;

/// Behaviour ids of the reflex catalog, for validating `inhibit`.
pub fn reflex_ids() -> &'static [&'static str] {
    static IDS: OnceLock<Vec<&'static str>> = OnceLock::new();
    IDS.get_or_init(|| {
        let mut ids: Vec<&'static str> = default_catalog().iter().map(|b| b.id()).collect();
        ids.retain(|id| *id != reflex_core::behaviours::GOAL_ID);
        ids
    })
}

pub fn scope_of<H: Host>(scope: WireScope, host: &mut H, snap: &Snapshot) -> Result<Scope, String> {
    Ok(match scope {
        WireScope::Everywhere => Scope::Everywhere,
        WireScope::Region { min, max } => {
            if min.iter().chain(&max).any(|v| !v.is_finite()) {
                return Err("region corners must be finite numbers".into());
            }
            let lo = Vec3::new(min[0].min(max[0]), min[1].min(max[1]), min[2].min(max[2]));
            let hi = Vec3::new(min[0].max(max[0]), min[1].max(max[1]), min[2].max(max[2]));
            Scope::Region(Aabb { min: lo, max: hi })
        }
        WireScope::Here { radius } => {
            if !(radius.is_finite() && radius > 0.0) {
                return Err("radius must be a positive number".into());
            }
            let r = Vec3::new(radius, radius, radius);
            let p = snap.me.pos;
            Scope::Region(Aabb {
                min: p - r,
                max: p + r,
            })
        }
        WireScope::UntilGoalEnds => match &host.knobs().goal {
            Some(g) => Scope::UntilGoalEnds(g.id),
            None => return Err("until_goal_ends needs an active goal".into()),
        },
    })
}

pub fn ticks(ttl_s: Option<f64>) -> Result<Option<u64>, String> {
    match ttl_s {
        None => Ok(None),
        Some(s) if s.is_finite() && s > 0.0 => Ok(Some((s * TICKS_PER_S).round().max(1.0) as u64)),
        Some(_) => Err("ttl_s must be a positive number of seconds".into()),
    }
}
