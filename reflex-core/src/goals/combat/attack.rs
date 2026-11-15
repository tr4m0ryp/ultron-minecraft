use super::melee::engage;
use crate::goals::ports::Navigator;
use crate::goals::spec::AttackTarget;
use crate::goals::task::{GoalCx, GoalTask, Plan, Step};
use crate::goals::travel::Walker;
use crate::snapshot::{Entity, EntityClass, EntityKind};

pub const SIGHT: f64 = 24.0;
/// Ticks a locked target may be out of view before the goal ends.
const GONE_TICKS: u32 = 40;
/// A target last seen this close that vanishes has died (it is well inside the sensing
/// radius), whoever landed the blow: live, the melee reflex D2 killed the zombie while it
/// held the body, and the goal reported "lost sight" with the zombie dead.
const DIED_WITHIN: f64 = 12.0;

pub struct Attack {
    target: AttackTarget,
    locked: Option<u32>,
    swings: u32,
    missing: u32,
    last_dist: f64,
    walker: Walker,
    items_at_start: u32,
    started: bool,
}

impl Attack {
    pub fn new(target: AttackTarget) -> Self {
        Self {
            target,
            locked: None,
            swings: 0,
            missing: 0,
            last_dist: f64::INFINITY,
            walker: Walker::default(),
            items_at_start: 0,
            started: false,
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
        // A dying target (death animation, drops already out) counts as gone.
        if let Some(id) = self.locked {
            return snap.entity(id).filter(|e| !e.state.dying);
        }
        match &self.target {
            AttackTarget::Id(id) => snap.entity(*id),
            AttackTarget::Kind(k) => snap
                .nearby(SIGHT, |e| kind_fits(*k, e) && !e.state.dying)
                .into_iter()
                .next(),
        }
    }

    /// What the kill put in the inventory (drops land under the body at melee range and
    /// are picked up during the fight), so thought does not go looking for them.
    fn loot(&self, cx: &GoalCx) -> String {
        let got = cx.world.total_items().saturating_sub(self.items_at_start);
        match got {
            0 => String::new(),
            1 => "; picked up 1 item".into(),
            n => format!("; picked up {n} items"),
        }
    }
}

/// `passive` names every animal; a species names only itself.
fn kind_fits(k: EntityKind, e: &Entity) -> bool {
    e.kind == k || (k == EntityKind::Passive && e.kind.class() == EntityClass::Passive)
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
        if !self.started {
            self.started = true;
            self.items_at_start = cx.world.total_items();
        }
        let Some(e) = self.find(cx).cloned() else {
            if self.locked.is_none() {
                return Step::Failed(format!("no {} in view", self.describe()));
            }
            if !cx.preempted {
                self.missing += 1;
            }
            if self.missing > GONE_TICKS {
                let loot = self.loot(cx);
                return if self.swings > 0 {
                    Step::Done(format!(
                        "{} gone after {} hits{loot}",
                        self.describe(),
                        self.swings
                    ))
                } else if self.last_dist <= DIED_WITHIN {
                    Step::Done(format!("{} is dead{loot}", self.describe()))
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
        self.last_dist = e.pos.dist(cx.me_pos());
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
