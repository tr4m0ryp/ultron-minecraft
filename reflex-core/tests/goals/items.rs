use crate::sim::Sim;
use reflex_core::channels::MainCmd;
use reflex_core::BlockPos;

#[test]
fn give_walks_to_the_companion_and_tosses() {
    let mut s = Sim::new();
    s.companion_at(9.5, 3.5);
    s.item("bread", 5, Some(4));
    s.goal("give bread 2", false);
    assert!(s.run_until(200, |s| s.has_event("goal_done")));
    let w = s.world.state();
    assert_eq!(w.tossed, vec![("bread".to_string(), 2)]);
    assert_eq!(w.inventory, vec![("bread".to_string(), 3)]);
    drop(w);
    assert!(s.snap.me.pos.dist(s.snap.companion.as_ref().unwrap().pos) <= 4.0);
}

#[test]
fn give_fails_without_enough_items() {
    let mut s = Sim::new();
    s.companion_at(3.5, 0.5);
    s.item("bread", 1, Some(4));
    s.goal("give bread 3", false);
    assert!(s.run_until(10, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("only 1 bread"));
    assert!(s.world.state().tossed.is_empty());
}

#[test]
fn place_puts_the_block_and_checks_preconditions() {
    let mut s = Sim::new();
    s.item("cobblestone", 4, Some(13));
    s.snap.me.inventory.push((
        13,
        reflex_core::snapshot::ItemStack::new(reflex_core::snapshot::ItemKind::ThrowawayBlock, 4),
    ));
    let at = BlockPos::new(6, 64, 2);
    s.place_at = Some(at);
    s.goal("place cobblestone 6 64 2", true);
    assert!(s.run_until(200, |s| s.has_event("goal_done")));
    assert_eq!(
        s.world.state().blocks.get(&at).map(String::as_str),
        Some("cobblestone")
    );

    let mut s = Sim::new();
    s.block(at, "stone");
    s.item("cobblestone", 4, Some(1));
    s.goal("place cobblestone 6 64 2", true);
    assert!(s.run_until(10, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("occupied by stone"));

    let mut s = Sim::new();
    s.goal("place cobblestone 6 64 2", true);
    assert!(s.run_until(10, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("no cobblestone"));
}

#[test]
fn place_overwrites_ground_cover_like_leaf_litter() {
    let at = BlockPos::new(6, 64, 2);
    for cover in ["leaf_litter", "short_dry_grass", "bush", "vine"] {
        let mut s = Sim::new();
        s.block(at, cover);
        s.item("crafting_table", 1, Some(13));
        s.place_at = Some(at);
        s.goal("place crafting_table 6 64 2", true);
        assert!(
            s.run_until(200, |s| s.has_event("goal_done")),
            "{cover}: {}",
            s.goal_text("goal_failed")
        );
    }
}

#[test]
fn place_never_clicks_a_block_that_opens() {
    let at = BlockPos::new(6, 65, 2);
    let mut s = Sim::new();
    s.block(at.below(), "crafting_table");
    s.item("chest", 1, Some(13));
    s.place_at = Some(at);
    s.goal("place chest 6 65 2", true);
    assert!(s.run_until(200, |s| s.has_event("goal_failed")));
    assert!(
        s.goal_text("goal_failed").contains("would open it"),
        "{}",
        s.goal_text("goal_failed")
    );

    let mut s = Sim::new();
    s.block(at.below(), "crafting_table");
    s.block(at.offset(1, 0, 0), "stone");
    s.item("chest", 1, Some(13));
    s.place_at = Some(at);
    s.goal("place chest 6 65 2", true);
    assert!(s.run_until(200, |s| s.has_event("goal_done")));
}

#[test]
fn place_box_fills_every_open_cell_in_one_goal() {
    let mut s = Sim::new();
    s.item("oak_planks", 20, Some(13));
    s.place_on_top = true;
    // The sim's straight-line navigator cannot walk around its own new wall, so the row
    // stays within reach of the start.
    s.block(BlockPos::new(2, 64, 2), "stone");
    s.goal("place oak_planks 1 64 2 4 64 2", true);
    assert!(s.run_until(1500, |s| s.has_event("goal_done")));
    let blocks = s.world.state().blocks.clone();
    for x in [1, 3, 4] {
        assert_eq!(
            blocks.get(&BlockPos::new(x, 64, 2)).map(String::as_str),
            Some("oak_planks"),
            "x {x}"
        );
    }
    let text = s.goal_text("goal_done");
    assert!(text.contains("placed 3 oak_planks"), "{text}");
    assert!(text.contains("1 cells already held a block"), "{text}");
}

#[test]
fn place_box_stops_when_the_blocks_run_out() {
    let mut s = Sim::new();
    s.item("oak_planks", 2, Some(13));
    s.place_on_top = true;
    s.goal("place oak_planks 1 64 2 4 64 2", true);
    assert!(s.run_until(1500, |s| s.has_event("goal_failed")));
    let text = s.goal_text("goal_failed");
    assert!(text.contains("ran out of oak_planks; placed 2"), "{text}");
}

#[test]
fn place_climbs_to_see_a_roof_support_above_the_eyes() {
    // Live: a two-high wall, the body below, every roof block refused with "stand
    // higher" and nothing acted on it. Now the skill picks a raised spot that sees the
    // wall top and asks the route planner to get there (on a pillar).
    let at = BlockPos::new(3, 66, 2);
    let mut s = Sim::new();
    s.block(BlockPos::new(3, 64, 2), "oak_planks");
    s.block(BlockPos::new(3, 65, 2), "oak_planks");
    s.item("oak_planks", 4, Some(13));
    s.item("dirt", 8, Some(14));
    s.place_at = Some(at);
    s.goal("place oak_planks 3 66 2", true);
    let mut raised = None;
    for _ in 0..100 {
        s.step();
        if let Some(reflex_core::goals::NavTarget::Block { pos, .. }) = s.nav.state().last {
            raised = Some(pos);
            break;
        }
    }
    let spot = raised.unwrap_or_else(|| panic!("walks to a spot: {:?}", s.events("goal_failed")));
    assert!(spot.y >= 65, "eye above the wall top: {spot:?}");
}

#[test]
fn a_fill_takes_down_the_scaffold_its_walks_put_up() {
    // Live: a placement inside a fill dropped its scaffold list when it finished, so the
    // fill never took the scaffold down and the next cell did not know what it stood on.
    let mut s = Sim::new();
    s.item("oak_planks", 8, Some(13));
    s.place_on_top = true;
    let scaffold = BlockPos::new(8, 64, 5);
    s.block(scaffold, "dirt");
    s.nav.state().placed.push(scaffold);
    s.goal("place oak_planks 9 64 2 10 64 2", true);
    assert!(
        s.run_until(1500, |s| s.has_event("goal_done")),
        "{}",
        s.goal_text("goal_failed")
    );
    let text = s.goal_text("goal_done");
    assert!(text.contains("took down 1 scaffold blocks"), "{text}");
    assert_eq!(s.world.state().blocks.get(&scaffold), None);
}

#[test]
fn a_roof_is_never_laid_from_under_it() {
    // Live: laying a hut roof the body stood inside the walls, was roofed in, and dug out
    // under its own wall to reach the edge cells. Every spot it walks to is now outside the
    // footprint or on top of the roof.
    let mut s = Sim::new();
    for x in 2..=6 {
        for z in 2..=6 {
            if x == 2 || x == 6 || z == 2 || z == 6 {
                s.block(BlockPos::new(x, 64, z), "dirt");
                s.block(BlockPos::new(x, 65, z), "dirt");
            }
        }
    }
    s.item("dirt", 64, Some(13));
    s.goal("place dirt 2 66 2 6 66 6", true);
    let mut spots = Vec::new();
    for _ in 0..4000 {
        s.step();
        if let Some(reflex_core::goals::NavTarget::Block { pos, .. }) = s.nav.state().last {
            if spots.last() != Some(&pos) {
                spots.push(pos);
            }
        }
        if s.has_event("goal_done") || s.has_event("goal_failed") {
            break;
        }
    }
    assert!(!spots.is_empty(), "the body walks to a spot");
    let under: Vec<_> = spots
        .iter()
        .filter(|p| (2..=6).contains(&p.x) && (2..=6).contains(&p.z) && p.y < 66)
        .collect();
    // The test planner walks level lines only, so the raised spots are never reached and
    // the fill itself is measured live; what counts here is where the body is sent.
    assert!(under.is_empty(), "stood under the roof at {under:?}");
    assert!(
        spots.iter().all(|p| p.y >= 65),
        "eye above the wall top: {spots:?}"
    );
}

#[test]
fn a_fill_never_digs_at_a_scaffold_cell_that_holds_no_block() {
    // Live: a pillar cell stayed on the fill's scaffold list with no block in it, and the
    // hand dug at the empty cell until the fill gave that cell up (60 ticks, and before
    // that bound for good: one hut build hung for 4 minutes).
    let mut s = Sim::new();
    s.item("oak_planks", 8, Some(13));
    s.place_on_top = true;
    let ghost = BlockPos::new(2, 64, 0);
    s.block(BlockPos::new(2, 64, 1), "dirt");
    s.block(BlockPos::new(2, 65, 1), "dirt");
    s.nav.state().placed.push(ghost);
    s.goal("place oak_planks 2 66 0 2 66 1", true);
    let mut dug_at_air = 0;
    for _ in 0..1500 {
        let out = s.step();
        let empty = !s.world.state().blocks.contains_key(&ghost);
        if out.main == Some(MainCmd::Dig(ghost)) && empty {
            dug_at_air += 1;
        }
        if s.has_event("goal_done") || s.has_event("goal_failed") {
            break;
        }
    }
    assert_eq!(dug_at_air, 0, "ticks spent digging at a cell with no block");
}
