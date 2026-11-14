//! Regressions for bugs found running against a real 26.3 server.

mod common;

use common::*;
use reflex_core::behaviours::hazard::FallingBlocks;
use reflex_core::snapshot::BlockKind;
use reflex_core::{BlockPos, Priority};

/// Head inside leaves (or glass, or a slab) does no damage in vanilla; R9 used to treat any
/// impassable head block as suffocation and held the legs at P0 forever.
#[test]
fn r9_ignores_non_suffocating_head_block() {
    let mut s = flat();
    let head = BlockPos::new(0, 65, 0);
    s.blocks.set(head, BlockKind::SolidOpen);
    assert!(eval(&mut FallingBlocks, &s, &knobs()).0.is_none());
    s.blocks.set(head, BlockKind::Solid);
    let bid = eval(&mut FallingBlocks, &s, &knobs()).0.expect("stone suffocates");
    assert_eq!(bid.priority, Priority::P0);
}

#[test]
fn solid_open_supports_but_is_not_passable() {
    assert!(BlockKind::SolidOpen.supports());
    assert!(!BlockKind::SolidOpen.passable());
    assert!(!BlockKind::SolidOpen.suffocates());
}
