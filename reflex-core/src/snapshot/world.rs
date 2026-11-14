use std::collections::HashMap;

use crate::math::BlockPos;

/// Blocks reduced to the classes the reflexes care about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BlockKind {
    #[default]
    Unknown,
    Air,
    Solid,
    /// Collides like a solid block but never suffocates: leaves, glass, and blocks whose
    /// collision box is not a full cube (slabs, stairs, fences, snow layers).
    SolidOpen,
    /// Solid but subject to gravity: sand, gravel, concrete powder.
    Gravity,
    Water,
    Lava,
    Fire,
    Campfire,
    Cactus,
    BerryBush,
    WitherRose,
    Magma,
    PowderSnow,
    /// Ladder, vine, scaffolding: passable and climbable.
    Climbable,
    /// Negates fall damage on landing: slime, cobweb, hay counts as partial (treated as full here).
    Cushion,
}

impl BlockKind {
    /// Can a player stand on top of this block.
    pub fn supports(self) -> bool {
        matches!(
            self,
            BlockKind::Solid
                | BlockKind::SolidOpen
                | BlockKind::Gravity
                | BlockKind::Magma
                | BlockKind::Cactus
                | BlockKind::Campfire
                | BlockKind::Cushion
        )
    }

    /// Can the player's body occupy this block.
    pub fn passable(self) -> bool {
        matches!(
            self,
            BlockKind::Air
                | BlockKind::Water
                | BlockKind::Lava
                | BlockKind::Fire
                | BlockKind::BerryBush
                | BlockKind::WitherRose
                | BlockKind::PowderSnow
                | BlockKind::Climbable
        )
    }

    /// Damages a player whose box touches it.
    pub fn contact_hazard(self) -> bool {
        matches!(
            self,
            BlockKind::Lava
                | BlockKind::Fire
                | BlockKind::Campfire
                | BlockKind::Cactus
                | BlockKind::BerryBush
                | BlockKind::WitherRose
                | BlockKind::PowderSnow
        )
    }

    /// A head inside this block takes suffocation damage (full opaque cubes only).
    pub fn suffocates(self) -> bool {
        matches!(
            self,
            BlockKind::Solid | BlockKind::Gravity | BlockKind::Magma | BlockKind::Cushion
        )
    }

    pub fn is_liquid(self) -> bool {
        matches!(self, BlockKind::Water | BlockKind::Lava)
    }

    /// Landing on or in this block negates fall damage.
    pub fn negates_fall(self) -> bool {
        matches!(
            self,
            BlockKind::Water | BlockKind::Cushion | BlockKind::Climbable | BlockKind::PowderSnow
        )
    }
}

/// Sparse map of the blocks around the player. Positions not present are `Unknown`
/// (unloaded or outside the sensing radius), which the reflexes treat as untrusted.
#[derive(Clone, Debug, Default)]
pub struct LocalBlocks {
    map: HashMap<BlockPos, BlockKind>,
}

impl LocalBlocks {
    pub fn get(&self, p: BlockPos) -> BlockKind {
        self.map.get(&p).copied().unwrap_or_default()
    }

    pub fn set(&mut self, p: BlockPos, k: BlockKind) {
        self.map.insert(p, k);
    }

    /// Fill an inclusive box with one kind; convenient for tests and for the adapter.
    pub fn fill(&mut self, a: BlockPos, b: BlockPos, k: BlockKind) {
        for x in a.x.min(b.x)..=a.x.max(b.x) {
            for y in a.y.min(b.y)..=a.y.max(b.y) {
                for z in a.z.min(b.z)..=a.z.max(b.z) {
                    self.map.insert(BlockPos::new(x, y, z), k);
                }
            }
        }
    }

    /// A player can stand with feet in `p`: body space free and harmless, support below.
    pub fn standable(&self, p: BlockPos) -> bool {
        let (feet, head, floor) = (self.get(p), self.get(p.above()), self.get(p.below()));
        feet.passable()
            && !feet.contact_hazard()
            && !feet.is_liquid()
            && head.passable()
            && !head.contact_hazard()
            && floor.supports()
            && floor != BlockKind::Magma
            && floor != BlockKind::Cactus
            && floor != BlockKind::Campfire
    }

    /// Highest block at or below `from.y` in the column that supports or stops a fall,
    /// scanning at most `depth` blocks. Returns its position and kind.
    pub fn floor_below(&self, from: BlockPos, depth: i32) -> Option<(BlockPos, BlockKind)> {
        (0..=depth).map(|d| from.offset(0, -d, 0)).find_map(|p| {
            let k = self.get(p);
            (k.supports() || k.negates_fall() || k == BlockKind::Lava || k == BlockKind::Unknown)
                .then_some((p, k))
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = (&BlockPos, &BlockKind)> {
        self.map.iter()
    }
}
