//! The craft and smelt goals over the fixture recipe book: planned chains, stations found
//! or put down, furnace visits, and the one-shot shopping list.

use std::sync::Arc;

use crate::sim::Sim;
use reflex_core::goals::fakes::recipes::book;
use reflex_core::goals::{MenuStatus, World};
use reflex_core::BlockPos;

fn sim() -> Sim {
    let s = Sim::new();
    s.world.state().book = Arc::new(book());
    s
}

fn held(s: &Sim, item: &str) -> u32 {
    s.world.count_item(item)
}

/// The outcome text without its "#1 craft: " subject prefix.
fn said(s: &Sim, kind: &str) -> String {
    let t = s.goal_text(kind);
    t.split_once(": ").map_or(t.clone(), |(_, r)| r.to_string())
}

fn placed(s: &Sim, block: &str) -> Vec<BlockPos> {
    let w = s.world.state();
    w.blocks
        .iter()
        .filter(|(_, n)| *n == block)
        .map(|(p, _)| *p)
        .collect()
}

#[test]
fn craft_in_the_inventory_grid() {
    let mut s = sim();
    s.item("oak_log", 3, None);
    s.world.state().craft_ticks = 3;
    s.goal("craft oak_planks 8", true);
    assert!(s.run_until(80, |s| s.has_event("goal_done")));
    assert_eq!((held(&s, "oak_log"), held(&s, "oak_planks")), (1, 8));
}

#[test]
fn a_stone_pickaxe_from_logs_puts_its_own_table_down() {
    let mut s = sim();
    s.item("oak_log", 2, Some(1));
    s.item("cobblestone", 3, Some(2));
    s.goal("craft stone_pickaxe", true);
    assert!(
        s.run_until(600, |s| s.has_event("goal_done")),
        "{:?}",
        s.events("goal_failed")
    );
    assert_eq!(held(&s, "stone_pickaxe"), 1);
    let tables = placed(&s, "crafting_table");
    assert_eq!(tables.len(), 1, "one table put down");
    let t = tables[0];
    let text = said(&s, "goal_done");
    let at = format!("put down a crafting_table at {} {} {}", t.x, t.y, t.z);
    assert!(text.contains(&at), "{text}");
    assert_eq!((held(&s, "oak_log"), held(&s, "cobblestone")), (0, 0));
}

#[test]
fn a_table_in_reach_is_walked_to_not_crafted() {
    let mut s = sim();
    s.item("birch_planks", 3, None);
    s.item("stick", 2, None);
    s.block(BlockPos::new(10, 64, 0), "crafting_table");
    s.goal("craft wooden_axe", true);
    assert!(s.run_until(300, |s| s.has_event("goal_done")));
    assert!(s.snap.me.pos.x > 6.0, "walked to the table");
    assert_eq!(placed(&s, "crafting_table").len(), 1);
}

#[test]
fn missing_raw_materials_fail_once_with_the_shopping_list() {
    let mut s = sim();
    s.item("cobblestone", 1, None);
    s.goal("craft stone_pickaxe", true);
    assert!(s.run_until(40, |s| s.has_event("goal_failed")));
    let text = said(&s, "goal_failed");
    assert!(
        text.starts_with("cannot craft stone_pickaxe: need 1 crafting_table"),
        "{text}"
    );
    assert!(text.ends_with("Get 2 any log, 2 cobblestone/blackstone/cobbled_deepslate."));
    s.run_until(40, |_| false);
    assert_eq!(s.events("goal_failed").len(), 1, "no retries");
    let mut none = sim();
    none.goal("craft beacon", true);
    assert!(none.run_until(40, |s| s.has_event("goal_failed")));
    assert_eq!(said(&none, "goal_failed"), "no recipe makes beacon");
}

#[test]
fn adapter_errors_retry_then_fail() {
    let mut s = sim();
    s.item("oak_log", 1, None);
    s.world.state().craft_error = Some("menu closed".into());
    s.goal("craft oak_planks", true);
    assert!(
        s.run_until(60, |s| s.has_event("goal_done")),
        "one error is retried"
    );
    let mut w = Sim::new();
    assert_eq!(World::craft_status(&mut w.world), MenuStatus::Idle);
    let mut empty = Sim::new();
    empty.goal("craft stick", true);
    assert!(empty.run_until(40, |s| s.has_event("goal_failed")));
    assert!(said(&empty, "goal_failed").starts_with("no recipe data"));
}

#[test]
fn an_iron_pickaxe_chain_smelts_at_a_placed_furnace() {
    let mut s = sim();
    s.item("raw_iron", 3, Some(1));
    s.item("coal", 1, Some(2));
    s.item("oak_log", 3, Some(3));
    s.item("cobblestone", 8, Some(4));
    s.goal("craft iron_pickaxe", true);
    assert!(
        s.run_until(3000, |s| s.has_event("goal_done")),
        "{:?}",
        s.events("goal_failed")
    );
    assert_eq!(held(&s, "iron_pickaxe"), 1);
    assert_eq!(placed(&s, "furnace").len(), 1);
    assert_eq!(placed(&s, "crafting_table").len(), 1);
    assert_eq!((held(&s, "raw_iron"), held(&s, "coal")), (0, 0));
    let text = said(&s, "goal_done");
    assert!(
        text.contains("a crafting_table at") && text.contains("a furnace at"),
        "{text}"
    );
}

#[test]
fn smelt_goal_waits_out_the_cooking_and_collects() {
    let mut s = sim();
    s.item("raw_iron", 2, None);
    s.item("oak_planks", 2, None);
    s.block(BlockPos::new(3, 64, 0), "furnace");
    s.goal("smelt iron_ingot 2", true);
    assert!(
        !s.run_until(300, |s| s.has_event("goal_done")),
        "still cooking at 15 s"
    );
    assert!(s.run_until(400, |s| s.has_event("goal_done")));
    assert_eq!(said(&s, "goal_done"), "smelted 2 iron_ingot");
    assert_eq!((held(&s, "iron_ingot"), held(&s, "oak_planks")), (2, 0));
}
