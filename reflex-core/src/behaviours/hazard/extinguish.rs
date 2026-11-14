use crate::behaviour::{Behaviour, Bid, Ctx, Priority, Veto};
use crate::behaviours::common::heading_to;
use crate::channels::{Commands, GazeCmd, LegsCmd, MainCmd, Speed};
use crate::math::BlockPos;
use crate::snapshot::{BlockKind, ItemKind};

/// R3: on fire, not in water, no fire resistance. Step into water within 4 blocks; else
/// place a water bucket at the feet and retrieve it; else stop sprinting (a veto).
#[derive(Default)]
pub struct Extinguish {
    placed_at: Option<u64>,
}

const WATER_RADIUS: i32 = 4;
const RETRIEVE_AFTER: u64 = 3;

fn burning(ctx: &Ctx) -> bool {
    let me = &ctx.snap.me;
    me.on_fire && !me.in_water && !me.fire_resistance
}

fn nearest_water(ctx: &Ctx) -> Option<BlockPos> {
    let feet = ctx.snap.me.pos.block();
    let r = WATER_RADIUS;
    let mut best: Option<(f64, BlockPos)> = None;
    for dx in -r..=r {
        for dy in -1..=1 {
            for dz in -r..=r {
                let p = feet.offset(dx, dy, dz);
                if ctx.snap.blocks.get(p) != BlockKind::Water {
                    continue;
                }
                let d = p.feet_centre().dist(ctx.snap.me.pos);
                if d <= r as f64 && best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, p));
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

impl Behaviour for Extinguish {
    fn id(&self) -> &'static str {
        "R3"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let me = &ctx.snap.me;
        let down = GazeCmd::Angles {
            yaw: me.yaw,
            pitch: 90.0,
        };
        if let Some(t) = self.placed_at {
            let held_empty = me.held().is_some_and(|s| s.kind == ItemKind::EmptyBucket);
            if ctx.snap.tick >= t + RETRIEVE_AFTER && held_empty {
                self.placed_at = None;
                let cmds = Commands::default().gaze(down).main(MainCmd::Use);
                return Some(Bid::new(Priority::P1, 0.8, "bucket_retrieve", cmds));
            }
            if !held_empty {
                self.placed_at = None;
            }
            return None;
        }
        if !burning(ctx) {
            return None;
        }
        if let Some(w) = nearest_water(ctx) {
            let legs = LegsCmd::toward(heading_to(me.pos, w.feet_centre()), Speed::Sprint);
            return Some(Bid::new(
                Priority::P1,
                0.9,
                "extinguish_water",
                Commands::default().legs(legs),
            ));
        }
        let slot = me.hotbar_find(|k| k == ItemKind::WaterBucket)?;
        let cmds = if me.selected_slot == slot {
            self.placed_at = Some(ctx.snap.tick);
            Commands::default().gaze(down).main(MainCmd::Use)
        } else {
            Commands::default()
                .gaze(down)
                .main(MainCmd::SelectSlot(slot))
        };
        Some(Bid::new(Priority::P1, 0.9, "extinguish_bucket", cmds).lock(2))
    }

    fn veto(&mut self, ctx: &Ctx, plan: &mut Commands) -> Option<Veto> {
        if !burning(ctx) || nearest_water(ctx).is_some() {
            return None;
        }
        let legs = plan.legs.as_mut().filter(|l| l.speed == Speed::Sprint)?;
        legs.speed = Speed::Walk;
        Some(Veto {
            priority: Priority::P1,
            note: "burning_no_sprint",
        })
    }
}
