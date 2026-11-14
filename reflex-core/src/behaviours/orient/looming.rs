use std::collections::HashMap;

use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::channels::Commands;
use crate::predict::tau;
use crate::snapshot::{EntityClass, EntityKind};

/// O2: an entity within 16 blocks whose time to contact is under 40 ticks (scaled by
/// vigilance). Turn to it; hostiles bid at P2, others at P4. Harmless kinds get inhibition
/// of return (O6): not re-oriented to for 200 ticks.
#[derive(Default)]
pub struct Looming {
    ignore_until: HashMap<EntityKind, u64>,
}

const RANGE: f64 = 16.0;
const BASE_TAU: f64 = 40.0;
const RETURN_INHIBIT: u64 = 200;

impl Behaviour for Looming {
    fn id(&self) -> &'static str {
        "O2"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let snap = ctx.snap;
        let me = &snap.me;
        let threshold = BASE_TAU * (0.5 + ctx.knobs.gains.vigilance as f64);
        let now = snap.tick;
        let companion = snap.companion.as_ref().and_then(|c| c.entity_id);
        let best = snap
            .entities
            .iter()
            .filter(|e| Some(e.id) != companion)
            .filter(|e| !matches!(e.kind.class(), EntityClass::Item | EntityClass::Projectile | EntityClass::FallingBlock))
            .filter(|e| e.pos.dist(me.pos) <= RANGE)
            .filter(|e| self.ignore_until.get(&e.kind).is_none_or(|t| now >= *t))
            .filter_map(|e| Some((e, tau(me.pos, me.vel, e.pos, e.vel)?)))
            .filter(|(_, t)| *t < threshold)
            .min_by(|a, b| a.1.total_cmp(&b.1))?;
        let (e, t) = best;
        let hostile = e.kind.class() == EntityClass::Hostile;
        if !hostile {
            self.ignore_until.insert(e.kind, now + RETURN_INHIBIT);
        }
        let priority = if hostile { Priority::P2 } else { Priority::P4 };
        let urgency = (1.0 - t / threshold) as f32;
        Some(Bid::new(priority, urgency.max(0.1), "orient_looming", Commands::default().look_at(e.head())))
    }
}
