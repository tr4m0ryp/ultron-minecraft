use super::melee::engage;
use crate::goals::ports::Navigator;
use crate::goals::spec::AttackTarget;
use crate::goals::task::{GoalCx, GoalTask, Plan, Step};
use crate::goals::travel::Walker;
use crate::snapshot::{Entity, EntityClass};

pub const SIGHT: f64 = 24.0;
/// Ticks a locked target may be out of view before the goal ends.
const GONE_TICKS: u32 = 40;

pub struct Attack {
    target: AttackTarget,
    locked: Option<u32>,
    swings: u32,
    missing: u32,
    walker: Walker,
}

impl Attack {
    pub fn new(target: AttackTarget) -> Self {
        Self {
            target,
            locked: None,
            swings: 0,
            missing: 0,
            walker: Walker::default(),
        }
    }

    fn describe(&self) -> String {
        match &self.target {
            AttackTarget::Id(id) => format!("entity #{id}"),
            AttackTarget::Kind(k) => k.label().to_string(),
        }
    }

    fn find<'s>(&self, cx: &'s GoalCx) -> Option<&'s Entity> {
        let snap = cx.snap();
        if let Some(id) = self.locked {
            return snap.entity(id);
        }
        match &self.target {
            AttackTarget::Id(id) => snap.entity(*id),
            AttackTarget::Kind(k) => snap.nearby(SIGHT, |e| e.kind == *k).into_iter().next(),
        }
    }
}

/// The companion and protected kinds are off limits unless the brain lifted the D11 mask.
pub fn masked(cx: &GoalCx, e: &Entity) -> bool {
    let snap = cx.snap();
    let companion = snap.companion.as_ref().and_then(|c| c.entity_id) == Some(e.id);
    let protected = companion || e.kind.class() == EntityClass::Protected;
    protected && cx.knobs().inhibition_for("D11", snap.me.pos, snap.tick) <= 0.0
}

impl GoalTask for Attack {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        let Some(e) = self.find(cx).cloned() else {
            if self.locked.is_none() {
                return Step::Failed(format!("no {} in view", self.describe()));
            }
            if !cx.preempted {
                self.missing += 1;
            }
            if self.missing > GONE_TICKS {
                return if self.swings > 0 {
                    Step::Done(format!(
                        "{} gone after {} hits",
                        self.describe(),
                        self.swings
                    ))
                } else {
                    Step::Failed(format!("lost sight of the {}", self.describe()))
                };
            }
            return Step::Bid(Plan::hold("goal_attack"));
        };
        if masked(cx, &e) {
            return Step::Failed(format!(
                "{} is protected by the attack mask (inhibit D11 to allow)",
                e.kind.label()
            ));
        }
        self.locked = Some(e.id);
        self.missing = 0;
        let (plan, swing) = engage(cx, &mut self.walker, &e);
        if swing && !cx.preempted {
            self.swings += 1;
        }
        Step::Bid(plan)
    }

    fn stop(&mut self, _nav: &mut dyn Navigator) {
        self.walker.reset();
    }
}
