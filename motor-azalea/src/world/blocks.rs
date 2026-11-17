//! Block reads and the nearest-block search.

use std::collections::HashSet;

use azalea::{
    block::{
        fluid_state::{FluidKind, FluidState},
        BlockStates,
    },
    registry::builtin::BlockKind,
    Client,
};
use reflex_core::math::BlockPos;

use super::names::{block_kinds, block_name};

/// `find_blocks` yields in rough `x+y+z` order; stop once candidates are this many blocks
/// (Manhattan) past the radius, or after this many candidates.
const MANHATTAN_SLACK: i32 = 16;
const MAX_CANDIDATES: usize = 512;

fn az(p: BlockPos) -> azalea::BlockPos {
    azalea::BlockPos::new(p.x, p.y, p.z)
}

pub fn block_at(bot: &Client, p: BlockPos) -> Option<String> {
    let world = bot.world().ok()?;
    let w = world.read();
    w.get_block_state(az(p)).map(block_name)
}

pub fn nearest(
    bot: &Client,
    kind: &str,
    from: BlockPos,
    radius: i32,
    exclude: &[BlockPos],
) -> Option<BlockPos> {
    let kinds: HashSet<_> = block_kinds(kind).into_iter().collect();
    if kinds.is_empty() {
        return None;
    }
    let states = BlockStates::from(kinds);
    let world = bot.world().ok()?;
    let w = world.read();
    let r2 = (radius * radius) as i64;
    let mut best: Option<(i64, BlockPos)> = None;
    for (i, p) in w.find_blocks(az(from), &states).enumerate() {
        let (dx, dy, dz) = (p.x - from.x, p.y - from.y, p.z - from.z);
        if dx.abs() + dy.abs() + dz.abs() > 2 * radius + MANHATTAN_SLACK || i > MAX_CANDIDATES {
            break;
        }
        let q = BlockPos::new(p.x, p.y, p.z);
        let d2 = (dx * dx + dy * dy + dz * dz) as i64;
        if d2 <= r2 && !exclude.contains(&q) && best.is_none_or(|(bd, _)| d2 < bd) {
            best = Some((d2, q));
        }
    }
    best.map(|(_, p)| p)
}

/// The nearest water source block (not flowing water: a bottle fills at a source only)
/// with air over it, within `radius` of `from`.
pub fn water_source(bot: &Client, from: BlockPos, radius: i32) -> Option<BlockPos> {
    let states = BlockStates::from(HashSet::from([BlockKind::Water]));
    let world = bot.world().ok()?;
    let w = world.read();
    let r2 = (radius * radius) as i64;
    let mut best: Option<(i64, BlockPos)> = None;
    for (i, p) in w.find_blocks(az(from), &states).enumerate() {
        let (dx, dy, dz) = (p.x - from.x, p.y - from.y, p.z - from.z);
        if dx.abs() + dy.abs() + dz.abs() > 2 * radius + MANHATTAN_SLACK || i > MAX_CANDIDATES {
            break;
        }
        let d2 = (dx * dx + dy * dy + dz * dz) as i64;
        if d2 > r2 || best.is_some_and(|(bd, _)| d2 >= bd) {
            continue;
        }
        let source = w.get_block_state(p).is_some_and(|s| {
            let f = FluidState::from(s);
            f.kind == FluidKind::Water && f.amount == 8 && !f.falling
        });
        let open = w.get_block_state(p.up(1)).is_some_and(|s| s.is_air());
        if source && open {
            best = Some((d2, BlockPos::new(p.x, p.y, p.z)));
        }
    }
    best.map(|(_, p)| p)
}
