//! Getting into position for a placement: pick a spot whose eye sees a face of a support,
//! walk there, and step onto its middle when the face is still hidden from where the walk
//! ended.

use super::{stand, Place};
use crate::goals::ports::NavTarget;
use crate::goals::task::{GoalCx, Plan, Step};
use crate::goals::travel::route::SCAFFOLD;
use crate::goals::travel::Walk;

/// Standing spots tried before the cell counts as out of reach.
const MAX_SPOTS: u32 = 3;
/// Ticks the body may take to step onto the middle of a spot.
const SETTLE_TICKS: u32 = 30;

impl Place {
    fn scaffold_held(cx: &GoalCx) -> u32 {
        SCAFFOLD.iter().map(|b| cx.world.count_item(b)).sum()
    }

    /// Walk to a spot that sees a face of a support.
    pub(super) fn position(&mut self, cx: &mut GoalCx) -> Step {
        if self.spot.is_none() {
            if self.spots_tried >= MAX_SPOTS {
                return Step::Failed(format!("no spot found that sees a face at {}", self.at()));
            }
            let held = Self::scaffold_held(cx);
            let bars = stand::Bars {
                keep_clear: &self.keep_clear,
                tried: &self.tried,
                cover: &self.cover,
            };
            self.spot = stand::find(cx, &self.block, self.pos, bars, held);
            self.settle = 0;
            self.spots_tried += 1;
            if self.spot.is_none() {
                return Step::Failed(format!(
                    "nowhere to stand within reach of {} that sees a face to place against",
                    self.at()
                ));
            }
        }
        let spot = self.spot.expect("set above");
        let t = NavTarget::Block {
            pos: spot.feet,
            radius: 0.5,
        };
        match self.walker.walk(cx, t, "goal_place_walk") {
            Walk::Moving(p) => Step::Bid(p),
            // Arrived and the face is still hidden: step onto the middle of the spot, where
            // it was seen from; hidden from there too (the world changed), another spot.
            Walk::Arrived => {
                let centre = stand::onto_centre(cx.me_pos(), spot.feet);
                if let Some(c) = centre.filter(|_| self.settle < SETTLE_TICKS) {
                    self.settle += u32::from(!cx.preempted);
                    return Step::Bid(Plan::new("goal_place", c));
                }
                self.tried.push(spot.feet);
                self.spot = None;
                Step::Bid(Plan::hold("goal_place"))
            }
            Walk::Failed(_) => {
                self.tried.push(spot.feet);
                self.spot = None;
                Step::Bid(Plan::hold("goal_place"))
            }
        }
    }
}
