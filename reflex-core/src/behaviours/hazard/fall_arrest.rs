use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::channels::{Commands, GazeCmd, MainCmd};
use crate::events::EventClass;
use crate::math::{angle_diff, yaw_pitch_to, BlockPos, Vec3};
use crate::snapshot::{BlockKind, ItemKind, Snapshot};

/// Survival block interaction range from the eye.
const REACH: f64 = 4.5;
/// A placed water source is scooped back up within this many ticks of placing it.
const RETRIEVE_TICKS: u64 = 40;
/// The crosshair must be this close (degrees) to the clutch face before placing, because
/// the client places on whatever face is under the crosshair.
const AIM_TOLERANCE: f32 = 4.0;

/// A block clutch: the ledge block beside the air cell under our centre, and the centre of
/// its face toward that cell. Placing on that face fills the cell we are falling into. The
/// cell's top must be at or below the feet so the new block does not intersect the body.
fn clutch_target(snap: &Snapshot) -> Option<(BlockPos, Vec3)> {
    let me = &snap.me;
    let col = me.pos.block();
    (1..=3).find_map(|drop| {
        let cell = BlockPos::new(col.x, col.y - drop, col.z);
        let k = snap.blocks.get(cell);
        if !k.passable() || k.is_liquid() {
            return None;
        }
        [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().find_map(|&(dx, dz)| {
            let ledge = cell.offset(dx, 0, dz);
            if !snap.blocks.get(ledge).supports() {
                return None;
            }
            let face = Vec3::new(
                ledge.x as f64 + 0.5 - dx as f64 * 0.5,
                ledge.y as f64 + 0.5,
                ledge.z as f64 + 0.5 - dz as f64 * 0.5,
            );
            (me.eye().dist(face) <= REACH).then_some((ledge, face))
        })
    })
}

/// R6 fall arrest: airborne with a predicted landing that would take us below hp_floor.
/// With a throwaway block and a ledge beside the cell below: place a block under the feet
/// (block clutch). Else with a water bucket: pre-select it and look straight down as soon as
/// triggered, place water on the landing block once the predicted ticks to landing reach
/// the `mlg_lead` gain (measured on the 26.3 server), then scoop it back up after landing.
#[derive(Default)]
pub struct FallArrest {
    /// Tick and cell of the last water placement, for the one-shot rule and the retrieve.
    placed: Option<(u64, BlockPos)>,
}

impl FallArrest {
    fn retrieve(&mut self, ctx: &Ctx) -> Option<Bid> {
        let (t, cell) = self.placed?;
        let me = &ctx.snap.me;
        if ctx.snap.tick > t + RETRIEVE_TICKS {
            self.placed = None;
            return None;
        }
        let holding_empty = me.held().is_some_and(|s| s.kind == ItemKind::EmptyBucket);
        if !(me.on_ground || me.in_water)
            || !holding_empty
            || ctx.snap.blocks.get(cell) != BlockKind::Water
        {
            return None;
        }
        let aim = Vec3::new(
            cell.x as f64 + 0.5,
            cell.y as f64 + 0.5,
            cell.z as f64 + 0.5,
        );
        let cmds = Commands::default().look_at(aim).main(MainCmd::Use);
        Some(Bid::new(Priority::P3, 0.8, "water_retrieve", cmds))
    }
}

impl Behaviour for FallArrest {
    fn id(&self) -> &'static str {
        "R6"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        if let Some(bid) = self.retrieve(ctx) {
            return Some(bid);
        }
        let snap = ctx.snap;
        let me = &snap.me;
        if me.on_ground || me.in_water {
            return None;
        }
        let fall = ctx.pred.fall?;
        let threshold = (me.hp_total() - ctx.knobs.gains.hp_floor).max(1.0);
        if fall.into_void || fall.damage < threshold || fall.landing.negates_fall() {
            return None;
        }
        let block = me.hotbar_find(|k| k == ItemKind::ThrowawayBlock);
        if let (Some(slot), Some((ledge, face))) = (block, clutch_target(snap)) {
            let look = Commands::default().look_at(face);
            if me.selected_slot != slot {
                let cmds = look.main(MainCmd::SelectSlot(slot));
                return Some(Bid::new(Priority::P0, 1.0, "block_clutch", cmds));
            }
            let (yaw, pitch) = yaw_pitch_to(me.eye(), face);
            let aimed = angle_diff(me.yaw, yaw).abs() <= AIM_TOLERANCE
                && (me.pitch - pitch).abs() <= AIM_TOLERANCE;
            let cmds = if aimed {
                look.main(MainCmd::PlaceOn(ledge))
            } else {
                look
            };
            return Some(Bid::new(Priority::P0, 1.0, "block_clutch", cmds));
        }
        let slot = me.hotbar_find(|k| k == ItemKind::WaterBucket)?;
        let down = Commands::default().gaze(GazeCmd::Angles {
            yaw: me.yaw,
            pitch: 90.0,
        });
        if me.selected_slot != slot {
            let cmds = down.main(MainCmd::SelectSlot(slot));
            return Some(Bid::new(Priority::P0, 1.0, "fall_arrest", cmds));
        }
        let floor = fall.floor.filter(|_| fall.landing.supports())?;
        let fresh = self
            .placed
            .is_none_or(|(t, _)| snap.tick > t + RETRIEVE_TICKS);
        if !fresh || fall.t_land > ctx.knobs.gains.mlg_lead {
            return Some(Bid::new(Priority::P0, 1.0, "fall_arrest", down));
        }
        let top = Vec3::new(me.pos.x, floor.y as f64 + 1.0, me.pos.z);
        self.placed = Some((snap.tick, floor.above()));
        ctx.emit(
            ctx.report(EventClass::Log, "water_place", "R6").text(format!(
                "t_land={} eye_dist={:.2} vy={:.3} y={:.2}",
                fall.t_land,
                me.eye().dist(top),
                me.vel.y,
                me.pos.y
            )),
        );
        Some(Bid::new(
            Priority::P0,
            1.0,
            "water_place",
            down.main(MainCmd::Use),
        ))
    }
}
