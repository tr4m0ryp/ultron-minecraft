//! Hand-built snapshots and helpers shared by the integration tests.
#![allow(dead_code)]

use reflex_core::behaviour::{Behaviour, Bid, Ctx, Veto};
use reflex_core::channels::{Commands, LegsCmd, Speed};
use reflex_core::events::Event;
use reflex_core::knobs::Knobs;
use reflex_core::predict::{Efference, Prediction};
use reflex_core::snapshot::{BlockKind, Entity, EntityKind, ItemKind, ItemStack, Snapshot};
use reflex_core::{BlockPos, Vec3};

pub const BOT: &str = "Ultron";
pub const FLOOR_Y: i32 = 63;

/// Flat stone floor at y=63 with air above, player standing at (0.5, 64, 0.5).
pub fn flat() -> Snapshot {
    let mut s = Snapshot {
        tick: 1000,
        world_min_y: -64,
        ..Default::default()
    };
    let r = 12;
    s.blocks.fill(
        BlockPos::new(-r, FLOOR_Y - 8, -r),
        BlockPos::new(r, FLOOR_Y, r),
        BlockKind::Solid,
    );
    s.blocks.fill(
        BlockPos::new(-r, FLOOR_Y + 1, -r),
        BlockPos::new(r, FLOOR_Y + 4, r),
        BlockKind::Air,
    );
    s.me.pos = Vec3::new(0.5, 64.0, 0.5);
    s
}

/// Removes the floor for x >= `edge_x`, leaving a drop of `depth` blocks onto stone,
/// or open down to the void when `depth` is `None`.
pub fn cliff(s: &mut Snapshot, edge_x: i32, depth: Option<i32>) {
    let r = 12;
    let bottom = depth.map_or(s.world_min_y - 1, |d| FLOOR_Y - d);
    s.blocks.fill(
        BlockPos::new(edge_x, bottom + 1, -r),
        BlockPos::new(r, FLOOR_Y, r),
        BlockKind::Air,
    );
    if depth.is_some() {
        s.blocks.fill(
            BlockPos::new(edge_x, bottom, -r),
            BlockPos::new(r, bottom, r),
            BlockKind::Solid,
        );
    }
}

pub fn mob(id: u32, kind: EntityKind, x: f64, z: f64) -> Entity {
    Entity::new(id, kind, Vec3::new(x, 64.0, z))
}

pub fn give(s: &mut Snapshot, slot: usize, kind: ItemKind, count: u8) {
    s.me.hotbar[slot] = Some(ItemStack::new(kind, count));
}

pub fn bread() -> ItemKind {
    ItemKind::Food {
        nutrition: 5,
        saturation: 6.0,
    }
}

/// Evaluates one behaviour in isolation and returns its bid plus the events it emitted.
pub fn eval(b: &mut dyn Behaviour, s: &Snapshot, k: &Knobs) -> (Option<Bid>, Vec<Event>) {
    eval_eff(b, s, k, Efference::default())
}

pub fn eval_eff(
    b: &mut dyn Behaviour,
    s: &Snapshot,
    k: &Knobs,
    eff: Efference,
) -> (Option<Bid>, Vec<Event>) {
    let pred = Prediction::compute(s, eff);
    let ctx = Ctx::new(s, &pred, k, BOT);
    let bid = b.evaluate(&ctx);
    (bid, ctx.take_events())
}

/// Runs one behaviour's veto over `plan`.
pub fn veto(b: &mut dyn Behaviour, s: &Snapshot, k: &Knobs, plan: &mut Commands) -> Option<Veto> {
    let pred = Prediction::compute(s, Efference::default());
    let ctx = Ctx::new(s, &pred, k, BOT);
    b.veto(&ctx, plan)
}

pub fn walk(dir: Vec3) -> Commands {
    Commands::default().legs(LegsCmd::toward(dir, Speed::Walk))
}

pub fn knobs() -> Knobs {
    Knobs::new()
}
