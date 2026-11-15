//! Placing into occupied cells, the explicit break goal, and the ledger of the player's own
//! blocks keeping routes, escapes and mining away from them.

use crate::sim::Sim;
use reflex_core::goals::runner::ENCLOSED_TICKS;
use reflex_core::goals::travel::route::Region;
use reflex_core::goals::NavTarget;
use reflex_core::BlockPos;

const AT: BlockPos = BlockPos::new(6, 64, 2);

fn placing(s: &mut Sim, block: &str, spec: &str) {
    s.item(block, 4, Some(13));
    s.place_at = Some(AT);
    s.goal(spec, true);
}

fn block_at(s: &Sim, p: BlockPos) -> Option<String> {
    s.world.state().blocks.get(&p).cloned()
}

#[test]
fn place_clears_soft_natural_blocks_first() {
    for held in [
        "oak_leaves",
        "dirt",
        "grass_block",
        "sand",
        "gravel",
        "snow_block",
        "poppy",
    ] {
        let mut s = Sim::new();
        s.block(AT, held);
        placing(&mut s, "cobblestone", "place cobblestone 6 64 2");
        assert!(
            s.run_until(600, |s| s.has_event("goal_done")),
            "{held}: {}",
            s.goal_text("goal_failed")
        );
        assert_eq!(block_at(&s, AT).as_deref(), Some("cobblestone"), "{held}");
    }
}

#[test]
fn a_placed_block_refuses_without_replace_and_goes_with_it() {
    let mut s = Sim::new();
    s.block(AT, "oak_planks");
    s.handle.ledger().remember(AT, "oak_planks");
    placing(&mut s, "cobblestone", "place cobblestone 6 64 2");
    assert!(s.run_until(60, |s| s.has_event("goal_failed")));
    let why = s.goal_text("goal_failed");
    assert!(
        why.contains("a player placed") && why.contains("replace"),
        "{why}"
    );
    assert_eq!(block_at(&s, AT).as_deref(), Some("oak_planks"));

    let mut s = Sim::new();
    s.block(AT, "oak_planks");
    s.handle.ledger().remember(AT, "oak_planks");
    placing(&mut s, "cobblestone", "place cobblestone 6 64 2 replace");
    assert!(
        s.run_until(600, |s| s.has_event("goal_done")),
        "{}",
        s.goal_text("goal_failed")
    );
    assert_eq!(block_at(&s, AT).as_deref(), Some("cobblestone"));
}

#[test]
fn a_protected_region_refuses_without_replace_and_goes_with_it() {
    let region = Region::new(BlockPos::new(5, 60, 0), BlockPos::new(8, 70, 4));
    let mut s = Sim::new();
    s.block(AT, "dirt");
    s.exec.knobs.protect(region);
    placing(&mut s, "cobblestone", "place cobblestone 6 64 2");
    assert!(s.run_until(60, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("protected region"));
    assert_eq!(block_at(&s, AT).as_deref(), Some("dirt"));

    let mut s = Sim::new();
    s.block(AT, "dirt");
    s.exec.knobs.protect(region);
    placing(&mut s, "cobblestone", "place cobblestone 6 64 2 replace");
    assert!(s.run_until(600, |s| s.has_event("goal_done")));
}

#[test]
fn hard_ground_stays_unless_replace_is_given() {
    let mut s = Sim::new();
    s.block(AT, "stone");
    placing(&mut s, "cobblestone", "place cobblestone 6 64 2");
    assert!(s.run_until(60, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("occupied by stone"));

    let mut s = Sim::new();
    s.block(AT, "stone");
    placing(&mut s, "cobblestone", "place cobblestone 6 64 2 replace");
    assert!(s.run_until(600, |s| s.has_event("goal_done")));
}

#[test]
fn a_placement_the_server_took_is_remembered_as_the_players_own() {
    let mut s = Sim::new();
    placing(&mut s, "dirt", "place dirt 6 64 2");
    assert!(s.run_until(300, |s| s.has_event("goal_done")));
    assert_eq!(
        s.handle.ledger().own_cells(),
        vec![(AT, "dirt".to_string())]
    );
    // A cell that already held the block is not claimed: the player did not place it.
    let mut s = Sim::new();
    s.block(AT, "dirt");
    placing(&mut s, "dirt", "place dirt 6 64 2");
    assert!(s.run_until(60, |s| s.has_event("goal_done")));
    assert!(s.handle.ledger().is_empty());
}

#[test]
fn a_fill_clears_soft_cells_and_leaves_placed_ones() {
    let mut s = Sim::new();
    s.block(BlockPos::new(1, 64, 2), "oak_leaves");
    s.block(BlockPos::new(2, 64, 2), "oak_planks");
    s.handle
        .ledger()
        .remember(BlockPos::new(2, 64, 2), "oak_planks");
    s.item("cobblestone", 8, Some(13));
    s.place_on_top = true;
    s.goal("place cobblestone 1 64 2 3 64 2", true);
    assert!(
        s.run_until(2000, |s| s.has_event("goal_done")),
        "{}",
        s.goal_text("goal_failed")
    );
    assert_eq!(
        block_at(&s, BlockPos::new(1, 64, 2)).as_deref(),
        Some("cobblestone")
    );
    assert_eq!(
        block_at(&s, BlockPos::new(2, 64, 2)).as_deref(),
        Some("oak_planks")
    );
    assert_eq!(
        block_at(&s, BlockPos::new(3, 64, 2)).as_deref(),
        Some("cobblestone")
    );
    let text = s.goal_text("goal_done");
    assert!(text.contains("1 cells already held a block"), "{text}");
}

#[test]
fn break_digs_a_placed_block_on_purpose_and_the_ledger_lets_go() {
    let mut s = Sim::new();
    s.block(AT, "dirt");
    s.handle.ledger().remember(AT, "dirt");
    s.goal("break 6 64 2", true);
    assert!(
        s.run_until(600, |s| s.has_event("goal_done")),
        "{}",
        s.goal_text("goal_failed")
    );
    assert_eq!(block_at(&s, AT), None);
    assert!(!s.handle.ledger().is_placed(AT));
}

#[test]
fn mining_never_takes_a_block_the_player_placed() {
    let mut s = Sim::new();
    let hut = BlockPos::new(2, 64, 0);
    s.block(hut, "dirt");
    s.handle.ledger().remember(hut, "dirt");
    s.goal("mine dirt 1", true);
    assert!(s.run_until(300, |s| s.has_event("goal_failed")));
    assert!(
        s.goal_text("goal_failed").contains("no dirt"),
        "{}",
        s.goal_text("goal_failed")
    );
    assert_eq!(block_at(&s, hut).as_deref(), Some("dirt"));
}

#[test]
fn a_walk_asks_the_navigator_to_keep_the_players_blocks_and_the_companions_margin() {
    let mut s = Sim::new();
    let ledger = s.handle.ledger().clone();
    ledger.remember(BlockPos::new(4, 64, 4), "dirt");
    ledger.remember_companion(BlockPos::new(10, 64, 10), "cobblestone");
    s.goal("go_to 8 64 0", true);
    for _ in 0..5 {
        s.step();
    }
    let keep = s.nav.state().keep.clone();
    assert!(keep.contains(BlockPos::new(4, 64, 4)));
    assert!(
        keep.contains(BlockPos::new(12, 66, 8)),
        "margin around the companion's block"
    );
    assert!(!keep.contains(BlockPos::new(5, 64, 4)));
}

#[test]
fn the_escape_drive_sends_the_keep_set_with_its_digging_request() {
    let mut s = Sim::new();
    s.world.state().no_sky = true;
    s.nav.state().shut_in = true;
    let wall = BlockPos::new(1, 64, 0);
    s.handle.ledger().remember(wall, "dirt");
    assert!(s.run_until(ENCLOSED_TICKS + 200, |s| {
        matches!(s.nav.state().last, Some(NavTarget::Sky { dig: true, .. }))
    }));
    assert!(
        s.nav.state().keep.contains(wall),
        "escape digs around the hut"
    );
}
