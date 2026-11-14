use super::melee::engage;
use crate::goals::ports::{NavTarget, Navigator};
use crate::goals::spec::Person;
use crate::goals::task::{GoalCx, GoalTask, Step};
use crate::goals::travel::{person_label, resolve_person, Walk, Walker};
use crate::math::Vec3;
use crate::snapshot::{Entity, EntityClass};

/// Hostiles this close to the companion count as attacking them.
pub const GUARD_RADIUS: f64 = 6.0;
/// Chase a threat at most this far from the companion.
const TETHER: f64 = 16.0;
const STAND_BY: f64 = 3.0;

/// Open-ended: stay near the companion and fight whatever threatens them. Never targets
/// the companion or a protected kind, whatever the D11 inhibition says.
pub struct Guard {
    who: Person,
    engaged: Option<u32>,
    walker: Walker,
}

impl Guard {
    pub fn new(who: Person) -> Self {
        Self {
            who,
            engaged: None,
            walker: Walker::default(),
        }
    }

    fn threat(&self, cx: &GoalCx, ward_id: u32, ward: Vec3) -> Option<Entity> {
        let snap = cx.snap();
        let allowed = |e: &&Entity| {
            e.id != ward_id
                && e.kind.class() != EntityClass::Protected
                && e.kind.class() != EntityClass::Passive
                && e.pos.dist(ward) <= TETHER
        };
        if let Some(e) = self.engaged.and_then(|id| snap.entity(id)).filter(allowed) {
            return Some(e.clone());
        }
        let attacker = snap
            .damage
            .iter()
            .filter_map(|d| d.attacker)
            .filter_map(|id| snap.entity(id))
            .find(allowed);
        let near_ward = snap
            .entities
            .iter()
            .filter(|e| e.kind.class() == EntityClass::Hostile && e.pos.dist(ward) <= GUARD_RADIUS)
            .filter(allowed)
            .min_by(|a, b| a.pos.dist(ward).total_cmp(&b.pos.dist(ward)));
        attacker.or(near_ward).cloned()
    }
}

impl GoalTask for Guard {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        let Some((id, pos, _)) = resolve_person(cx, &self.who) else {
            self.walker.stop(cx);
            cx.blocked(format!("{} not visible", person_label(&self.who)));
            return Step::Idle;
        };
        if let Some(e) = self.threat(cx, id, pos) {
            if self.engaged != Some(e.id) {
                self.walker.stop(cx);
                cx.note(format!(
                    "engaging {} near {}",
                    e.kind.label(),
                    person_label(&self.who)
                ));
            }
            self.engaged = Some(e.id);
            return Step::Bid(engage(cx, &mut self.walker, &e).0);
        }
        self.engaged = None;
        if cx.me_pos().dist(pos) <= STAND_BY + 1.5 {
            self.walker.stop(cx);
            return Step::Idle;
        }
        let t = NavTarget::Entity {
            id,
            distance: STAND_BY,
        };
        match self.walker.walk(cx, t, "goal_guard") {
            Walk::Moving(p) => Step::Bid(p),
            Walk::Arrived => Step::Idle,
            Walk::Failed(f) => {
                cx.blocked(format!("cannot reach {}: {f:?}", person_label(&self.who)));
                Step::Idle
            }
        }
    }

    fn stop(&mut self, _nav: &mut dyn Navigator) {
        self.walker.reset();
        self.engaged = None;
    }
}
