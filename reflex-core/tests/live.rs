//! Regressions for bugs found running against a real 26.3 server.

mod common;

use common::*;
use reflex_core::behaviours::hazard::{ContactHazard, FallingBlocks, LavaWithdrawal};
use reflex_core::snapshot::BlockKind;
use reflex_core::{BlockPos, Priority, Vec3};

/// Head inside leaves (or glass, or a slab) does no damage in vanilla; R9 used to treat any
/// impassable head block as suffocation and held the legs at P0 forever.
#[test]
fn r9_ignores_non_suffocating_head_block() {
    let mut s = flat();
    let head = BlockPos::new(0, 65, 0);
    s.blocks.set(head, BlockKind::SolidOpen);
    assert!(eval(&mut FallingBlocks, &s, &knobs()).0.is_none());
    s.blocks.set(head, BlockKind::Solid);
    let bid = eval(&mut FallingBlocks, &s, &knobs())
        .0
        .expect("stone suffocates");
    assert_eq!(bid.priority, Priority::P0);
}

#[test]
fn solid_open_supports_but_is_not_passable() {
    assert!(BlockKind::SolidOpen.supports());
    assert!(!BlockKind::SolidOpen.passable());
    assert!(!BlockKind::SolidOpen.suffocates());
}

/// Live: the bot walked half out of a lava source, its centre left the lava column while its
/// box still overlapped it, and R1 let go of the legs. R1 and R4 must test the whole box.
#[test]
fn r1_r4_see_hazards_the_box_straddles() {
    let mut s = flat();
    s.me.pos = Vec3::new(1.1, 64.0, 0.5);
    s.blocks.set(BlockPos::new(0, 64, 0), BlockKind::Lava);
    let bid = eval(&mut LavaWithdrawal, &s, &knobs())
        .0
        .expect("box overlaps lava");
    assert_eq!(bid.priority, Priority::P0);
    assert!(
        bid.commands.legs.unwrap().heading.x > 0.0,
        "moves away from the lava"
    );

    let mut s = flat();
    s.me.pos = Vec3::new(1.1, 64.0, 0.5);
    s.blocks.set(BlockPos::new(0, 64, 0), BlockKind::Fire);
    assert!(
        eval(&mut ContactHazard, &s, &knobs()).0.is_some(),
        "box overlaps fire"
    );

    s.me.pos = Vec3::new(1.5, 64.0, 0.5);
    assert!(
        eval(&mut ContactHazard, &s, &knobs()).0.is_none(),
        "clear of the fire"
    );
}

/// Live: dropped into a lava source, the bot climbed out and stopped beside it; the lava
/// spread onto it again. Lava within R1's NEAR margin now bids P1 to walk clear.
#[test]
fn r1_walks_clear_of_adjacent_lava() {
    let mut s = flat();
    s.blocks.set(BlockPos::new(2, 64, 0), BlockKind::Lava);
    assert!(eval(&mut LavaWithdrawal, &s, &knobs()).0.is_none(), "one block clear");
    s.me.pos = Vec3::new(1.5, 64.0, 0.5);
    let bid = eval(&mut LavaWithdrawal, &s, &knobs()).0.expect("adjacent lava");
    assert_eq!(bid.priority, Priority::P1);
    assert!(bid.commands.legs.unwrap().heading.x < 0.0, "walks away from the lava");
}
