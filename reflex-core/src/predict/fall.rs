use crate::math::Vec3;
use crate::snapshot::{BlockKind, Snapshot};

pub const GRAVITY: f64 = 0.08;
pub const DRAG: f64 = 0.98;
const MAX_TICKS: u32 = 400;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FallPrediction {
    /// Ticks until the feet reach the landing surface.
    pub t_land: u32,
    /// Predicted fall damage in HP after mitigation by the landing block.
    pub damage: f32,
    pub landing: BlockKind,
    /// Block whose top (or, for a non-supporting landing like water, whose volume) the feet
    /// reach; `None` over the void.
    pub floor: Option<crate::math::BlockPos>,
    /// The column has no floor above the void.
    pub into_void: bool,
}

/// Simulates vertical motion for an airborne player. `None` when on the ground or rising
/// with nothing to predict.
pub fn predict_fall(snap: &Snapshot) -> Option<FallPrediction> {
    let me = &snap.me;
    if me.on_ground || me.in_water {
        return None;
    }
    let column = me.pos.block();
    let depth = column.y - snap.world_min_y + 1;
    let floor = snap.blocks.floor_below(column.below(), depth.max(0));
    let Some((floor_pos, kind)) = floor else {
        return Some(FallPrediction {
            t_land: ticks_to_reach(me.pos.y, me.vel.y, (snap.world_min_y - 64) as f64),
            damage: f32::INFINITY,
            landing: BlockKind::Air,
            floor: None,
            into_void: true,
        });
    };
    let surface = if kind.supports() {
        floor_pos.y + 1
    } else {
        floor_pos.y
    } as f64;
    let t_land = ticks_to_reach(me.pos.y, me.vel.y, surface);
    let peak = peak_height(me.pos, me.vel.y);
    let fall_dist = me.fall_distance as f64 + (peak - surface).max(0.0);
    let damage = if kind.negates_fall() {
        0.0
    } else {
        fall_damage(fall_dist)
    };
    Some(FallPrediction {
        t_land,
        damage,
        landing: kind,
        floor: Some(floor_pos),
        into_void: false,
    })
}

/// Vanilla fall damage: one HP per block beyond three.
pub fn fall_damage(dist: f64) -> f32 {
    (dist - 3.0).ceil().max(0.0) as f32
}

fn ticks_to_reach(mut y: f64, mut vy: f64, target: f64) -> u32 {
    for t in 1..=MAX_TICKS {
        vy = (vy - GRAVITY) * DRAG;
        y += vy;
        if y <= target {
            return t;
        }
    }
    MAX_TICKS
}

fn peak_height(pos: Vec3, mut vy: f64) -> f64 {
    let mut y = pos.y;
    while vy > 0.0 {
        y += vy;
        vy = (vy - GRAVITY) * DRAG;
    }
    y
}
