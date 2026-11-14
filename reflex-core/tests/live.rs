//! Regressions for bugs found running against a real 26.3 server.

mod common;

use common::*;
use reflex_core::behaviours::defence::MeleeDefence;
use reflex_core::behaviours::hazard::{ContactHazard, FallingBlocks, LavaWithdrawal};
use reflex_core::behaviours::homeostasis::Health;
use reflex_core::behaviours::locomotion::Unstuck;
use reflex_core::channels::MainCmd;
use reflex_core::channels::{LegsCmd, Speed};
use reflex_core::predict::Efference;
use reflex_core::snapshot::{BlockKind, EntityKind};
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
    assert!(
        eval(&mut LavaWithdrawal, &s, &knobs()).0.is_none(),
        "one block clear"
    );
    s.me.pos = Vec3::new(1.5, 64.0, 0.5);
    let bid = eval(&mut LavaWithdrawal, &s, &knobs())
        .0
        .expect("adjacent lava");
    assert_eq!(bid.priority, Priority::P1);
    assert!(
        bid.commands.legs.unwrap().heading.x < 0.0,
        "walks away from the lava"
    );
}

/// Live: at hp 7 a zombie chased the bot into a platform corner; H2 and D2 kept bidding a
/// retreat that R5 held at the edge, the bot never swung, and the zombie knocked it off.
/// Cornered with the foe in reach, H2 releases the legs and D2 fights back.
#[test]
fn cornered_low_hp_fights_back() {
    let mut s = flat();
    s.blocks.fill(
        BlockPos::new(1, -64, -12),
        BlockPos::new(12, FLOOR_Y, 12),
        BlockKind::Air,
    );
    s.blocks.fill(
        BlockPos::new(-12, -64, 1),
        BlockPos::new(12, FLOOR_Y, 12),
        BlockKind::Air,
    );
    s.me.health = 6.0;
    s.entities.push(mob(7, EntityKind::Zombie, -1.0, -1.0));

    let bid = eval(&mut MeleeDefence, &s, &knobs()).0.expect("D2 bids");
    assert_eq!(bid.note, "melee_cornered");
    assert_eq!(bid.commands.main, Some(MainCmd::Attack(7)));
    let h2 = eval(&mut Health, &s, &knobs()).0;
    assert!(
        h2.is_none_or(|b| b.commands.legs.is_none()),
        "H2 leaves the legs"
    );

    // With open ground behind, the same situation is a retreat.
    let mut s = flat();
    s.me.health = 6.0;
    s.entities.push(mob(7, EntityKind::Zombie, -1.0, -1.0));
    let bid = eval(&mut MeleeDefence, &s, &knobs()).0.expect("D2 bids");
    assert_eq!(bid.note, "melee_retreat");
}

/// Live: walking east into a wall, C6 strafed, the strafe slid the bot along the wall, the
/// motion reset the stuck counter, and the goal walked it back into the wall: an endless
/// jump/strafe loop that never mined and never woke the brain.
#[test]
fn c6_sliding_along_a_wall_still_escalates_to_digging() {
    let mut s = flat();
    let east = LegsCmd::toward(Vec3::new(1.0, 0.0, 0.0), Speed::Walk);
    let mut b = Unstuck::default();
    let mut legs = east;
    let (mut dug, mut woke) = (false, false);
    for t in 0..260 {
        s.tick = 1000 + t;
        let from = s.me.pos;
        // The wall blocks x; any sideways input slides along it.
        if legs.heading.z.abs() > 0.1 {
            s.me.pos.z += legs.heading.z.signum() * 0.2;
        }
        let eff = Efference {
            legs: Some(legs),
            from,
        };
        let (bid, events) = eval_eff(&mut b, &s, &knobs(), eff);
        woke |= events.iter().any(|e| e.kind == "unstuck_failed");
        legs = match bid {
            Some(bid) => {
                dug |= matches!(bid.commands.main, Some(MainCmd::Dig(_)));
                bid.commands.legs.unwrap_or(east)
            }
            None => east,
        };
    }
    assert!(dug, "reached the dig stage");
    assert!(woke, "woke the brain");
}
