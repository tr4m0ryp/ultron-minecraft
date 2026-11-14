//! Block reads and the nearest-block search.

use std::collections::HashSet;

use azalea::{block::BlockStates, Client};
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

/// Squared distance, with blocks two or more below the feet made much dearer. Live, the
/// player mined stone in a ring under itself: the drops fell into pits it could not path
/// into (81 stone mined, 11 cobblestone picked up), and one pit trapped it for a stretch.
pub fn dig_cost(dx: i32, dy: i32, dz: i32) -> i64 {
    let d2 = i64::from(dx * dx + dy * dy + dz * dz);
    if dy < -1 {
        d2 + 15 * i64::from(dy * dy)
    } else {
        d2
    }
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
        let cost = dig_cost(dx, dy, dz);
        if d2 <= r2 && !exclude.contains(&q) && best.is_none_or(|(bc, _)| cost < bc) {
            best = Some((cost, q));
        }
    }
    best.map(|(_, p)| p)
}

#[cfg(test)]
mod tests {
    use super::dig_cost;

    #[test]
    fn level_or_higher_blocks_beat_a_pit_under_the_feet() {
        assert!(
            dig_cost(3, 0, 0) < dig_cost(0, -2, 0),
            "beside beats two down"
        );
        assert!(dig_cost(4, 2, 0) < dig_cost(1, -3, 0), "uphill beats a pit");
        assert_eq!(dig_cost(1, -1, 0), 2, "one down stays cheap");
    }
}
