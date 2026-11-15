use super::walker::{Walk, Walker};
use crate::goals::ports::{NavTarget, Navigator};
use crate::goals::spec::Person;
use crate::goals::task::{GoalCx, GoalTask, Step};
use crate::math::Vec3;

/// Start walking again once this much beyond the follow distance (hysteresis band).
const SLACK: f64 = 1.5;

/// Entity id, position and display name of a person the goal refers to.
pub fn resolve_person(cx: &GoalCx, who: &Person) -> Option<(u32, Vec3, String)> {
    let snap = cx.snap();
    let comp = snap.companion.as_ref();
    match who {
        Person::Companion => comp.and_then(|c| Some((c.entity_id?, c.pos, c.name.clone()))),
        Person::Named(n) => comp
            .filter(|c| c.name.eq_ignore_ascii_case(n))
            .and_then(|c| Some((c.entity_id?, c.pos, c.name.clone())))
            .or_else(|| {
                snap.entities
                    .iter()
                    .find(|e| e.name.as_deref().is_some_and(|m| m.eq_ignore_ascii_case(n)))
                    .map(|e| (e.id, e.pos, n.clone()))
            }),
    }
}

pub fn person_label(who: &Person) -> String {
    match who {
        Person::Companion => "companion".into(),
        Person::Named(n) => n.clone(),
    }
}

/// Open-ended: keeps within `distance` of the person, never finishes on its own.
pub struct Follow {
    who: Person,
    distance: f64,
    walker: Walker,
    moving: bool,
}

impl Follow {
    pub fn new(who: Person, distance: f64) -> Self {
        Self {
            who,
            distance: distance.max(1.0),
            walker: Walker::default(),
            moving: false,
        }
    }
}

impl GoalTask for Follow {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        let Some((id, pos, _)) = resolve_person(cx, &self.who) else {
            self.walker.stop(cx);
            cx.stuck(format!("{} not visible", person_label(&self.who)));
            return Step::Idle;
        };
        let d = cx.me_pos().dist(pos);
        if d <= self.distance {
            self.moving = false;
        } else if d > self.distance + SLACK {
            self.moving = true;
        }
        if !self.moving {
            self.walker.stop(cx);
            return Step::Idle;
        }
        let target = NavTarget::Entity {
            id,
            distance: self.distance,
        };
        match self.walker.walk(cx, target, "goal_follow") {
            Walk::Arrived => {
                self.moving = false;
                Step::Idle
            }
            Walk::Moving(plan) => Step::Bid(plan),
            Walk::Failed(f) => {
                cx.stuck(format!("cannot reach {}: {f:?}", person_label(&self.who)));
                Step::Idle
            }
        }
    }

    fn stop(&mut self, _nav: &mut dyn Navigator) {
        self.walker.reset();
    }
}
