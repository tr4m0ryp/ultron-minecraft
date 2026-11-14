use super::equip::equip;
use super::surfaces::{face_visible, opens_on_click, replaceable};
use crate::behaviours::common::away_from;
use crate::channels::{Commands, LegsCmd, MainCmd, Speed};
use crate::goals::ports::{kind_matches, NavTarget, Navigator};
use crate::goals::task::{GoalCx, GoalTask, Plan, Step};
use crate::goals::travel::{Walk, Walker};
use crate::math::{BlockPos, Vec3};

const REACH: f64 = 4.0;
/// Ticks between placement attempts, so the server's answer arrives before a retry.
const RETRY_GAP: u32 = 5;
const MAX_ATTEMPTS: u32 = 8;

pub struct Place {
    block: String,
    pos: BlockPos,
    walker: Walker,
    in_reach: bool,
    attempts: u32,
    wait: u32,
}

impl Place {
    pub fn new(block: String, pos: BlockPos) -> Self {
        Self {
            block,
            pos,
            walker: Walker::default(),
            in_reach: false,
            attempts: 0,
            wait: 0,
        }
    }

    fn at(&self) -> String {
        format!("{} {} {}", self.pos.x, self.pos.y, self.pos.z)
    }

    /// A non-replaceable neighbour to click on, below first. A block that opens on a click
    /// (table, chest, door) is never used: live, clicking the crafting table under a chest
    /// opened its menu instead, and the open menu then failed every later craft. With an
    /// `eye`, only a neighbour whose shared face that eye can see counts: live, every roof
    /// block was refused while the body stood inside the hut below the wall tops, because
    /// the top face it aimed at was above its eyes and the click landed on another face.
    fn support(&self, cx: &GoalCx, eye: Option<Vec3>) -> Result<BlockPos, String> {
        let p = self.pos;
        let mut opener = None;
        let mut hidden = false;
        for (x, y, z) in [
            (0, -1, 0),
            (1, 0, 0),
            (-1, 0, 0),
            (0, 0, 1),
            (0, 0, -1),
            (0, 1, 0),
        ] {
            let n = p.offset(x, y, z);
            match cx.world.block_at(n) {
                Some(k) if opens_on_click(&k) => opener = opener.or(Some(k)),
                Some(k) if !replaceable(&k) => {
                    if eye.is_none_or(|e| face_visible(e, n, p)) {
                        return Ok(n);
                    }
                    hidden = true;
                }
                _ => {}
            }
        }
        Err(match opener {
            _ if hidden => format!(
                "no face to place against at {} is visible from here; stand where you can \
                 see it (higher for a roof, or outside)",
                self.at()
            ),
            Some(k) => format!(
                "only a {k} to place against at {}; clicking it would open it",
                self.at()
            ),
            None => format!("nothing to place against at {}", self.at()),
        })
    }
}

impl GoalTask for Place {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        let current = cx.world.block_at(self.pos);
        if current
            .as_deref()
            .is_some_and(|n| kind_matches(&self.block, n))
        {
            return Step::Done(format!("placed {} at {}", self.block, self.at()));
        }
        if let Some(n) = current.as_deref().filter(|n| !replaceable(n)) {
            return Step::Failed(format!("{} is occupied by {n}", self.at()));
        }
        let Some(slot) = cx.world.item_slot(&self.block) else {
            return Step::Failed(format!("no {} to place", self.block));
        };
        if let Err(e) = self.support(cx, None) {
            return Step::Failed(e);
        }
        let centre = |b: BlockPos| b.feet_centre() + Vec3::new(0.0, 0.5, 0.0);
        if cx.me_pos().dist(centre(self.pos)) > REACH + 1.0 {
            self.in_reach = false;
        }
        if !self.in_reach {
            let t = NavTarget::Block {
                pos: self.pos,
                radius: REACH,
            };
            match self.walker.walk(cx, t, "goal_place_walk") {
                Walk::Arrived => self.in_reach = true,
                Walk::Moving(p) => return Step::Bid(p),
                Walk::Failed(f) => {
                    return Step::Failed(format!("cannot reach {}: {f:?}", self.at()))
                }
            }
        }
        let me = cx.snap().me.clone();
        let feet = me.pos.block();
        if feet == self.pos || feet.above() == self.pos {
            cx.blocked("standing in the spot to fill");
            let legs = LegsCmd::toward(away_from(me.pos, self.pos.feet_centre()), Speed::Walk);
            return Step::Bid(Plan::new("goal_place", Commands::default().legs(legs)));
        }
        if let Some(c) = equip(&me, slot) {
            return Step::Bid(Plan::new("goal_equip", c));
        }
        let support = match self.support(cx, Some(me.eye())) {
            Ok(b) => b,
            Err(e) => return Step::Failed(e),
        };
        let face = (centre(support) + centre(self.pos)) * 0.5;
        let aim = Commands::default().legs(LegsCmd::stop()).look_at(face);
        if self.wait > 0 {
            if !cx.preempted {
                self.wait -= 1;
            }
            return Step::Bid(Plan::new("goal_place", aim));
        }
        if self.attempts >= MAX_ATTEMPTS {
            return Step::Failed(format!(
                "the server did not accept a block at {}",
                self.at()
            ));
        }
        if !cx.preempted {
            self.attempts += 1;
            self.wait = RETRY_GAP;
        }
        Step::Bid(Plan::new("goal_place", aim.main(MainCmd::PlaceOn(support))))
    }

    fn stop(&mut self, _nav: &mut dyn Navigator) {
        self.walker.reset();
        self.in_reach = false;
    }
}
