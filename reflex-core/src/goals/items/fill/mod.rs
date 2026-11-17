//! Fills a box of cells with one block kind, one `Place` at a time, bottom layer first. Live,
//! the player built a hut with one place goal per block: about 60 thought turns, 4.5 s and
//! one model call each, which roughly doubled the stretch's cost per minute. A wall, floor
//! or roof is now one goal.

mod stray;

use super::place::{clearance, Cleanup, Clearance, Place};
use super::surfaces::replaceable;
use crate::goals::ports::Navigator;
use crate::goals::task::{GoalCx, GoalTask, Plan, Step};
use crate::math::BlockPos;

/// Most cells one fill goal may cover.
pub const FILL_MAX: usize = 64;
/// Cells that may fail before the whole fill gives up.
const MAX_FAILURES: usize = 4;

pub struct Fill {
    block: String,
    from: BlockPos,
    to: BlockPos,
    cells: Vec<BlockPos>,
    next: usize,
    current: Option<Place>,
    placed: u32,
    occupied: u32,
    replace: bool,
    failures: Vec<String>,
    cleanup: Cleanup,
    stray: stray::Budget,
    outcome: Option<Step>,
}

/// Cells of the box, bottom layer first, each layer walked in rows that turn back.
fn order(a: BlockPos, b: BlockPos) -> Vec<BlockPos> {
    let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
    let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
    let (z0, z1) = (a.z.min(b.z), a.z.max(b.z));
    let mut out = Vec::new();
    for y in y0..=y1 {
        for (row, x) in (x0..=x1).enumerate() {
            let zs: Vec<i32> = if row % 2 == 0 {
                (z0..=z1).collect()
            } else {
                (z0..=z1).rev().collect()
            };
            out.extend(zs.into_iter().map(|z| BlockPos::new(x, y, z)));
        }
    }
    out
}

impl Fill {
    pub fn new(block: String, from: BlockPos, to: BlockPos) -> Self {
        let mut cells = order(from, to);
        cells.truncate(FILL_MAX);
        Self {
            block,
            from,
            to,
            cells,
            next: 0,
            current: None,
            placed: 0,
            occupied: 0,
            replace: false,
            failures: Vec::new(),
            cleanup: Cleanup::default(),
            stray: stray::Budget::default(),
            outcome: None,
        }
    }

    /// `replace` lets every cell clear what is in it, placed or protected.
    pub fn replacing(mut self, replace: bool) -> Self {
        self.replace = replace;
        self
    }

    fn summary(&self) -> String {
        let p = |b: BlockPos| format!("{} {} {}", b.x, b.y, b.z);
        let mut s = format!(
            "placed {} {} in {} to {}",
            self.placed,
            self.block,
            p(self.from),
            p(self.to)
        );
        if self.occupied > 0 {
            s += &format!("; {} cells already held a block", self.occupied);
        }
        if let Some(last) = self.failures.last() {
            s += &format!("; {} cells failed, last: {last}", self.failures.len());
        }
        let c = &self.cleanup;
        if c.removed + c.left_standing > 0 {
            s += &format!("; took down {} scaffold blocks", c.removed);
            if c.left_standing > 0 {
                s += &format!(", {} left standing", c.left_standing);
            }
        }
        s
    }

    /// Takes the scaffold down, then reports `ok` or failure.
    fn finish(&mut self, cx: &mut GoalCx, ok: bool) -> Step {
        if let Some(p) = self.cleanup.step(cx) {
            return Step::Bid(p);
        }
        let prefix = match &self.outcome {
            Some(Step::Failed(p)) => p.clone(),
            _ => String::new(),
        };
        if ok && !(self.placed == 0 && !self.failures.is_empty()) {
            Step::Done(self.summary())
        } else {
            Step::Failed(prefix + &self.summary())
        }
    }
}

impl GoalTask for Fill {
    fn step(&mut self, cx: &mut GoalCx) -> Step {
        if let Some(Step::Failed(_)) = self.outcome {
            return self.finish(cx, false);
        }
        while self.current.is_none() {
            let Some(&pos) = self.cells.get(self.next) else {
                return self.finish(cx, true);
            };
            let held = cx.world.block_at(pos);
            // Soft natural blocks are cleared by the cell's own placement; a placed block,
            // a protected cell or hard ground is left, and counted as held.
            let refused = held.as_deref().is_some_and(|k| {
                !replaceable(k)
                    && matches!(clearance(cx, pos, k, self.replace), Clearance::Refuse(_))
            });
            match held {
                Some(k) if cx.world.matches(&self.block, &k) => self.next += 1,
                Some(_) if refused => {
                    self.occupied += 1;
                    self.next += 1;
                }
                _ if cx.world.item_slot(&self.block).is_none() => {
                    self.outcome = Some(Step::Failed(format!("ran out of {}; ", self.block)));
                    return self.finish(cx, false);
                }
                _ => {
                    let rest = self.cells[self.next + 1..].to_vec();
                    let place = Place::in_fill(self.block.clone(), pos, rest)
                        .replacing(self.replace)
                        .under(&self.cells)
                        .over_scaffold(self.cleanup.pending());
                    self.current = Some(place);
                }
            }
        }
        let place = self.current.as_mut().expect("set above");
        // A pillar the body climbed out on, now under the cells to fill: down it comes
        // while it can still be reached (see `stray`). Cells already empty leave the list
        // first, so a finished dig is never taken up again.
        self.cleanup.tally(cx);
        let mut scaffold = self.cleanup.pending().to_vec();
        scaffold.extend_from_slice(place.scaffold());
        scaffold.retain(|b| !self.stray.given_up(*b));
        if let Some(b) = stray::reachable(cx, &self.cells, &scaffold) {
            self.cleanup.add(place.release_scaffold(b));
            if self.stray.spend(b, cx.preempted) {
                return Step::Bid(self.cleanup.dig(cx, b));
            }
        }
        let step = place.step(cx);
        if !matches!(step, Step::Bid(_) | Step::Idle) {
            // A scaffold block a route put into one of the fill's own cells is part of the
            // build, not something to take down again.
            let scaffold = place.take_scaffold();
            let cells = &self.cells;
            self.cleanup
                .add(scaffold.into_iter().filter(|b| !cells.contains(b)));
        }
        match step {
            Step::Done(_) => {
                self.placed += 1;
                self.next += 1;
                self.current = None;
                let n = self.cells.len() as u32;
                let i = self.next as u32;
                cx.progress(i, n, format!("{i}/{n} cells, {} placed", self.placed));
                Step::Bid(Plan::hold("goal_place"))
            }
            Step::Failed(e) => {
                self.failures.push(e);
                self.next += 1;
                self.current = None;
                if self.failures.len() >= MAX_FAILURES {
                    self.outcome = Some(Step::Failed(String::new()));
                    return self.finish(cx, false);
                }
                Step::Bid(Plan::hold("goal_place"))
            }
            other => other,
        }
    }

    fn stop(&mut self, nav: &mut dyn Navigator) {
        if let Some(p) = self.current.as_mut() {
            p.stop(nav);
        }
        self.cleanup.reset_walk();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_is_bottom_up_and_turns_back_each_row() {
        let cells = order(BlockPos::new(1, 65, 1), BlockPos::new(0, 64, 0));
        let got: Vec<_> = cells.iter().map(|b| (b.x, b.y, b.z)).collect();
        assert_eq!(
            got,
            vec![
                (0, 64, 0),
                (0, 64, 1),
                (1, 64, 1),
                (1, 64, 0),
                (0, 65, 0),
                (0, 65, 1),
                (1, 65, 1),
                (1, 65, 0)
            ]
        );
    }
}
