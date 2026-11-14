use crate::math::BlockPos;

/// How the body gets from one standing cell to the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HopKind {
    /// Walk, step up, step or drop down, possibly after digging the way clear.
    Walk,
    /// Jump and place a block under the feet: the body ends one block higher, same column.
    Pillar,
    /// Place the missing floor ahead, then walk onto it.
    Bridge,
}

/// A block to place into `cell` by clicking the face of `against` that touches it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub cell: BlockPos,
    pub against: BlockPos,
}

/// One edge of a route: the work that opens it (blocks to break top-down, a block to
/// place) and the standing cell it ends on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hop {
    pub from: BlockPos,
    pub to: BlockPos,
    pub kind: HopKind,
    /// Blocks to break before moving, highest first so falling blocks never land in an
    /// opening the body is about to use.
    pub dig: Vec<BlockPos>,
    pub place: Option<Placement>,
}

impl Hop {
    pub fn walk(from: BlockPos, to: BlockPos) -> Self {
        Self {
            from,
            to,
            kind: HopKind::Walk,
            dig: Vec::new(),
            place: None,
        }
    }

    /// The hop needs the hands (digging or placing), not only the legs.
    pub fn has_work(&self) -> bool {
        !self.dig.is_empty() || self.place.is_some()
    }
}
