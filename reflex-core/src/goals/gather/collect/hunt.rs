//! Taking an item from a living animal: shear it, or kill it with the combat skills.

use super::source::Method;
use crate::behaviours::common::heading_to;
use crate::channels::{Commands, LegsCmd, MainCmd, Speed};
use crate::goals::combat::engage;
use crate::goals::items::equip;
use crate::goals::ports::NavTarget;
use crate::goals::task::{GoalCx, Plan};
use crate::goals::travel::{Walk, Walker};
use crate::snapshot::Snapshot;

/// Shears reach from here (entity interaction range is 3 from the eyes).
const SHEAR_REACH: f64 = 2.5;
/// Closer than this the body steers straight at the sheep instead of planning.
const DIRECT: f64 = 6.0;
const CLICK_GAP: u32 = 8;
const MAX_CLICKS: u32 = 4;
/// Ticks (not pre-empted) one animal may take before the hunt gives up on it.
const HUNT_LIMIT: u32 = 900;
/// A target last seen this close that vanishes has died, not wandered off.
const DIED_WITHIN: f64 = 12.0;

// One value per goal per tick, as `Step`; boxing the plan would buy nothing.
#[allow(clippy::large_enum_variant)]
pub enum HuntStep {
    Bid(Plan),
    /// The kill or the shearing happened; its drops are out.
    Done,
    /// Give up on this animal, with the reason.
    GiveUp(String),
    /// Shears in hand and in reach, but the sheep kept its wool.
    ShearFailed,
}

pub struct Hunt {
    pub id: u32,
    pub method: Method,
    clicks: u32,
    wait: u32,
    ticks: u32,
    last_dist: f64,
    name: &'static str,
}

impl Hunt {
    pub fn new(id: u32, method: Method) -> Self {
        Self {
            id,
            method,
            clicks: 0,
            wait: 0,
            ticks: 0,
            last_dist: 0.0,
            name: "animal",
        }
    }

    /// The animal died or was shorn, whoever's step saw it: drops may already be in reach.
    pub fn finished(&self, snap: &Snapshot) -> bool {
        match snap.entity(self.id) {
            None => self.last_dist <= DIED_WITHIN,
            Some(e) => e.state.dying || (self.method == Method::Shear && e.state.sheared),
        }
    }

    pub fn step(&mut self, cx: &mut GoalCx, walker: &mut Walker) -> HuntStep {
        let Some(e) = cx.snap().entity(self.id).cloned() else {
            return if self.last_dist <= DIED_WITHIN {
                HuntStep::Done
            } else {
                HuntStep::GiveUp(format!("lost sight of the {}", self.name))
            };
        };
        let name = e.kind.label();
        self.name = name;
        if e.state.dying || (self.method == Method::Shear && e.state.sheared) {
            return HuntStep::Done;
        }
        if !cx.preempted {
            self.ticks += 1;
        }
        if self.ticks > HUNT_LIMIT {
            return HuntStep::GiveUp(format!("could not get to the {name} in time"));
        }
        let me = cx.me_pos();
        self.last_dist = e.pos.dist(me);
        if self.method == Method::Kill {
            return HuntStep::Bid(engage(cx, walker, &e).0);
        }
        if self.last_dist > SHEAR_REACH {
            if self.last_dist <= DIRECT && super::straight_clear(cx.snap(), me, e.pos) {
                walker.stop(cx);
                let legs = LegsCmd::toward(heading_to(me, e.pos), Speed::Walk);
                let c = Commands::default().legs(legs).look_at(e.head());
                return HuntStep::Bid(Plan::new("goal_shear", c));
            }
            let t = NavTarget::Entity {
                id: e.id,
                distance: SHEAR_REACH - 0.5,
            };
            match walker.walk(cx, t, "goal_shear") {
                Walk::Moving(p) => return HuntStep::Bid(p),
                Walk::Arrived => {}
                Walk::Failed(f) => {
                    return HuntStep::GiveUp(format!("cannot reach the {name}: {f:?}"))
                }
            }
        }
        walker.stop(cx);
        let Some(slot) = cx.world.item_slot("shears") else {
            // The shears broke or left the inventory: kill instead.
            self.method = Method::Kill;
            return HuntStep::Bid(Plan::hold("goal_shear"));
        };
        let me = &cx.snap().me;
        if let Some(c) = equip(me, slot) {
            return HuntStep::Bid(Plan::new("goal_shear", c.look_at(e.head())));
        }
        let face = Commands::default().legs(LegsCmd::stop()).look_at(e.head());
        if self.wait > 0 {
            if !cx.preempted {
                self.wait -= 1;
            }
            return HuntStep::Bid(Plan::new("goal_shear", face));
        }
        if self.clicks >= MAX_CLICKS {
            return HuntStep::ShearFailed;
        }
        if !cx.preempted {
            self.clicks += 1;
            self.wait = CLICK_GAP;
        }
        HuntStep::Bid(Plan::new("goal_shear", face.main(MainCmd::Interact(e.id))))
    }
}
