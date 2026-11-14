use crate::sim::Sim;
use reflex_core::channels::{Channel, MainCmd};
use reflex_core::goals::{ItemDrop, ToolNeed};
use reflex_core::snapshot::{EntityKind, ItemKind};
use reflex_core::{BlockPos, Vec3};

fn ore_field(s: &mut Sim) {
    for (x, z) in [(8, 0), (9, 3), (-7, 6)] {
        s.block(BlockPos::new(x, 64, z), "iron_ore");
    }
    s.hotbar(2, ItemKind::Pickaxe, 1);
    s.world
        .state()
        .tools
        .insert("iron_ore".into(), ToolNeed::Have(2));
}

#[test]
fn mine_equips_the_tool_digs_each_block_and_finishes() {
    let mut s = Sim::new();
    ore_field(&mut s);
    s.goal("mine iron_ore 3", true);
    assert!(s.run_until(1500, |s| s.has_event("goal_done")));
    assert_eq!(s.snap.me.selected_slot, 2, "pickaxe selected");
    assert_eq!(s.world.state().blocks.len(), 0, "all three broken");
    assert!(s
        .goal_text("goal_done")
        .contains("mined 3 iron_ore, picked up 3 raw_iron"));
    let progress = s.events("goal_progress");
    assert!(!progress.is_empty() && progress.len() <= 3, "{progress:?}");
}

#[test]
fn mine_fails_without_tool_block_or_room() {
    let mut s = Sim::new();
    s.block(BlockPos::new(5, 64, 0), "diamond_ore");
    s.world.state().tools.insert(
        "diamond_ore".into(),
        ToolNeed::Missing("iron_pickaxe".into()),
    );
    s.goal("mine diamond_ore", true);
    assert!(s.run_until(20, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("missing tool"));

    let mut s = Sim::new();
    s.goal("mine gold_ore 2", true);
    assert!(s.run_until(20, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("no gold_ore within"));

    let mut s = Sim::new();
    ore_field(&mut s);
    s.world.state().free_slots = 0;
    s.goal("mine iron_ore", true);
    assert!(s.run_until(20, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("inventory full"));
}

#[test]
fn mine_skips_unreachable_blocks_then_fails() {
    let mut s = Sim::new();
    ore_field(&mut s);
    s.nav.state().unreachable = vec![
        BlockPos::new(8, 64, 0),
        BlockPos::new(9, 64, 3),
        BlockPos::new(-7, 64, 6),
    ];
    s.goal("mine iron_ore 3", true);
    assert!(s.run_until(200, |s| s.has_event("goal_failed")));
    assert!(s.has_event("goal_blocked"));
    assert!(s.goal_text("goal_failed").contains("no iron_ore within"));
}

#[test]
fn mine_is_preempted_by_melee_defence_and_resumes() {
    let mut s = Sim::new();
    ore_field(&mut s);
    s.goal("mine iron_ore 3", true);
    assert!(s.run_until(40, |s| s.exec.owner(Channel::Legs) == Some("GOAL")));
    let me = s.snap.me.pos;
    s.mob(7, EntityKind::Zombie, me.x - 4.0, me.z);
    assert!(s.run_until(60, |s| s.exec.owner(Channel::Legs) == Some("D2")));
    assert!(
        s.run_until(200, |s| s.snap.entity(7).is_none()),
        "D2 fought the zombie"
    );
    assert!(s.attacked.contains(&7));
    assert!(
        s.run_until(1500, |s| s.has_event("goal_done")),
        "mining resumed and finished"
    );
    assert!(s.nav.state().pauses >= 1);
}

#[test]
fn mine_never_digs_the_block_under_its_feet() {
    // Live bug: after a pickup step into a floor hole the nearest stone was the block under
    // the feet; R10 vetoed every dig and the goal burned its 600-tick dig limit per block.
    let mut s = Sim::new();
    let under = BlockPos::new(0, 63, 0);
    s.block(under, "stone");
    s.block(BlockPos::new(3, 64, 0), "stone");
    s.goal("mine stone 1", true);
    for _ in 0..60 {
        assert_ne!(s.step().main, Some(MainCmd::Dig(under)));
    }
    assert!(s.has_event("goal_done"));
    assert!(s.world.state().blocks.contains_key(&under), "floor kept");
}

#[test]
fn mine_digging_holds_gaze_and_main() {
    let mut s = Sim::new();
    s.block(BlockPos::new(2, 64, 0), "stone");
    s.goal("mine stone", true);
    let mut dug = false;
    for _ in 0..40 {
        if s.step().main == Some(MainCmd::Dig(BlockPos::new(2, 64, 0))) {
            dug = true;
        }
    }
    assert!(dug);
    assert!(s.has_event("goal_done"));
}

#[test]
fn collect_walks_over_drops_until_count() {
    let mut s = Sim::new();
    {
        let mut w = s.world.state();
        for (i, x) in [3.5, 9.5, -5.5].into_iter().enumerate() {
            w.drops.push(ItemDrop {
                entity_id: 50 + i as u32,
                item: "raw_iron".into(),
                count: 1,
                pos: Vec3::new(x, 64.0, 2.5),
            });
        }
    }
    s.goal("collect raw_iron 3", true);
    assert!(s.run_until(600, |s| s.has_event("goal_done")));
    assert_eq!(s.world.state().inventory, vec![("raw_iron".to_string(), 3)]);
}

#[test]
fn collect_plans_around_a_block_in_the_straight_line() {
    // Live bug: a pillar between the bot and a drop 6 blocks off; the direct walk pushed
    // into it and only the unstuck reflex moved the body.
    let mut s = Sim::new();
    s.snap.blocks.fill(
        reflex_core::BlockPos::new(2, 64, -1),
        reflex_core::BlockPos::new(2, 66, 1),
        reflex_core::snapshot::BlockKind::Solid,
    );
    s.world.state().drops.push(ItemDrop {
        entity_id: 50,
        item: "raw_iron".into(),
        count: 1,
        pos: Vec3::new(5.5, 64.0, 0.5),
    });
    s.goal("collect raw_iron 1", true);
    for _ in 0..5 {
        s.step();
    }
    assert!(
        s.nav.state().requests >= 1,
        "the planner routes around the pillar"
    );
}

#[test]
fn collect_fails_when_drops_run_out_or_inventory_full() {
    let mut s = Sim::new();
    s.world.state().drops.push(ItemDrop {
        entity_id: 50,
        item: "raw_iron".into(),
        count: 1,
        pos: Vec3::new(3.5, 64.0, 0.5),
    });
    s.goal("collect raw_iron 4", true);
    assert!(s.run_until(300, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("collected 1 of 4"));

    let mut s = Sim::new();
    s.world.state().free_slots = 0;
    s.goal("collect raw_iron", true);
    assert!(s.run_until(10, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").contains("inventory full"));
}

#[test]
fn mine_walks_to_drops_that_land_away_from_the_block() {
    // Live: logs fell off the trunk and cobblestone rolled into the pit; the old pickup
    // walked at the block for 2 s, gave up, and still reported the blocks as mined.
    let mut s = Sim::new();
    ore_field(&mut s);
    s.drop_offset = Some(Vec3::new(2.5, 0.0, 2.0));
    s.goal("mine iron_ore 2", true);
    assert!(s.run_until(2000, |s| s.has_event("goal_done")));
    assert_eq!(s.world.state().drops.len(), 0, "every drop picked up");
    assert!(
        s.goal_text("goal_done").contains("picked up 2 raw_iron"),
        "{}",
        s.goal_text("goal_done")
    );
}

#[test]
fn mine_counts_what_reached_the_inventory_not_what_broke() {
    // Drops out of reach never count: the goal mines on until the blocks run out and
    // says how many broke against how many arrived.
    let mut s = Sim::new();
    ore_field(&mut s);
    s.drop_offset = Some(Vec3::new(0.0, 3.0, 0.0));
    s.goal("mine iron_ore 1", true);
    assert!(s.run_until(3000, |s| s.has_event("goal_failed")));
    assert!(!s.has_event("goal_done"));
    assert!(
        s.goal_text("goal_failed")
            .contains("mined 3 iron_ore, picked up 0 raw_iron"),
        "{}",
        s.goal_text("goal_failed")
    );
}

#[test]
fn mine_tunnels_level_with_the_body_and_walks_over_the_drops() {
    // A 2-high stone seam ahead: the body breaks head then feet height, steps into the
    // gap and goes on, never digging the floor it stands on.
    let mut s = Sim::new();
    for x in 1..=4 {
        s.block(BlockPos::new(x, 64, 0), "stone");
        s.block(BlockPos::new(x, 65, 0), "stone");
    }
    s.goal("mine stone 6", true);
    let mut floor_dug = false;
    for _ in 0..3000 {
        if let Some(MainCmd::Dig(p)) = s.step().main {
            floor_dug |= p.y < 64;
        }
        if s.has_event("goal_done") {
            break;
        }
    }
    assert!(s.has_event("goal_done"), "{:?}", s.events("goal_failed"));
    assert!(!floor_dug);
    assert!(
        s.snap.me.pos.x >= 2.0,
        "advanced into the tunnel: {:?}",
        s.snap.me.pos
    );
    assert!(s.goal_text("goal_done").contains("picked up 6 cobblestone"));
}
