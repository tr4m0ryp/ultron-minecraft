//! Crafting stations: finding a crafting table or furnace within reach of a walk, and putting
//! a held one down beside the player with the place skill.

use crate::goals::items::place::Place;
use crate::goals::items::surfaces::replaceable;
use crate::goals::ports::Navigator;
use crate::goals::task::{GoalCx, GoalTask, Plan, Step};
use crate::math::BlockPos;

/// How far a station may stand and still be walked to.
pub const STATION_RADIUS: i32 = 16;
/// Spots tried before placing counts as failed.
const MAX_SPOTS: usize = 3;

/// The nearest block named exactly `name` ("furnace" must not find a blast_furnace).
pub fn find(cx: &GoalCx, name: &str) -> Option<BlockPos> {
    let from = cx.me_pos().block();
    let mut skip = Vec::new();
    for _ in 0..4 {
        let p = cx.world.nearest_block(name, from, STATION_RADIUS, &skip)?;
        if cx.world.block_at(p).as_deref() == Some(name) {
            return Some(p);
        }
        skip.push(p);
    }
    None
}

/// An open cell beside the player, on solid ground, not one it stands in: two blocks out
/// first (room to stand and face it), then adjacent, then three out.
fn spot(cx: &GoalCx, tried: &[BlockPos]) -> Option<BlockPos> {
    let feet = cx.me_pos().block();
    let open = |p: BlockPos| cx.world.block_at(p).is_some_and(|n| replaceable(&n));
    let solid = |p: BlockPos| cx.world.block_at(p).is_some_and(|n| !replaceable(&n));
    for r in [2, 1, 3] {
        for dy in [0, -1, 1] {
            for dx in -r..=r {
                for dz in -r..=r {
                    if dx.abs().max(dz.abs()) != r {
                        continue;
                    }
                    let p = BlockPos::new(feet.x + dx, feet.y + dy, feet.z + dz);
                    let mine = p == feet || p == feet.above();
                    if !mine && !tried.contains(&p) && open(p) && solid(p.below()) {
                        return Some(p);
                    }
                }
            }
        }
    }
    None
}

pub enum Placed {
    Bid(Plan),
    Done(BlockPos),
    Failed(String),
}

/// Putting one held station down nearby.
pub struct Placing {
    pub block: &'static str,
    place: Option<(BlockPos, Place)>,
    tried: Vec<BlockPos>,
}

impl Placing {
    pub fn new(block: &'static str) -> Self {
        Self {
            block,
            place: None,
            tried: Vec::new(),
        }
    }

    pub fn step(&mut self, cx: &mut GoalCx) -> Placed {
        if self.place.is_none() {
            if self.tried.len() >= MAX_SPOTS {
                return Placed::Failed(format!("found no spot to put a {} down", self.block));
            }
            let Some(p) = spot(cx, &self.tried) else {
                return Placed::Failed(format!("no open floor nearby for a {}", self.block));
            };
            self.tried.push(p);
            self.place = Some((p, Place::new(self.block.to_string(), p)));
        }
        let (p, place) = self.place.as_mut().expect("set above");
        match place.step(cx) {
            Step::Bid(plan) => Placed::Bid(plan),
            Step::Idle => Placed::Bid(Plan::hold("goal_craft_place")),
            Step::Done(_) => Placed::Done(*p),
            Step::Failed(why) => {
                if self.tried.len() >= MAX_SPOTS {
                    return Placed::Failed(format!("could not put a {} down: {why}", self.block));
                }
                self.place = None;
                Placed::Bid(Plan::hold("goal_craft_place"))
            }
        }
    }

    pub fn stop(&mut self, nav: &mut dyn Navigator) {
        if let Some((_, p)) = self.place.as_mut() {
            p.stop(nav);
        }
    }
}
