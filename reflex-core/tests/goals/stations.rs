//! The craft goal at a brewing stand and a smithing table, over the fixture recipe book:
//! stations found or put down, the 20 s a batch brews, fuel offered to the stand, and the
//! one-shot shopping list.

use std::sync::Arc;

use crate::sim::Sim;
use reflex_core::goals::fakes::recipes::book;
use reflex_core::goals::World;
use reflex_core::BlockPos;

fn sim() -> Sim {
    let s = Sim::new();
    s.world.state().book = Arc::new(book());
    s
}

fn held(s: &Sim, item: &str) -> u32 {
    s.world
        .inventory()
        .iter()
        .filter(|(n, _)| n == item)
        .map(|(_, c)| *c)
        .sum()
}

fn said(s: &Sim, kind: &str) -> String {
    let t = s.goal_text(kind);
    t.split_once(": ").map_or(t.clone(), |(_, r)| r.to_string())
}

const STAND: BlockPos = BlockPos::new(3, 64, 0);

#[test]
fn a_diamond_pickaxe_becomes_netherite_at_a_table_it_puts_down() {
    let mut s = sim();
    s.item("diamond_pickaxe", 1, Some(1));
    s.item("netherite_upgrade_smithing_template", 1, Some(2));
    s.item("netherite_ingot", 1, Some(3));
    s.item("smithing_table", 1, Some(4));
    s.goal("craft netherite_pickaxe", true);
    assert!(
        s.run_until(600, |s| s.has_event("goal_done")),
        "{:?}",
        s.events("goal_failed")
    );
    assert_eq!(
        (held(&s, "netherite_pickaxe"), held(&s, "diamond_pickaxe")),
        (1, 0)
    );
    assert_eq!(held(&s, "netherite_ingot"), 0);
    let text = said(&s, "goal_done");
    assert!(
        text.starts_with("smithed 1 netherite_pickaxe; put down a smithing_table at"),
        "{text}"
    );
}

#[test]
fn an_awkward_potion_then_a_healing_potion_on_one_blaze_powder() {
    let mut s = sim();
    s.block(STAND, "brewing_stand");
    s.item("water_potion", 1, Some(1));
    s.item("nether_wart", 1, Some(2));
    s.item("glistering_melon_slice", 1, Some(3));
    s.item("blaze_powder", 1, Some(4));
    s.goal("craft awkward_potion", true);
    assert!(
        !s.run_until(400, |s| s.has_event("goal_done")),
        "a batch brews for 20 s"
    );
    assert!(
        s.run_until(300, |s| s.has_event("goal_done")),
        "{:?}",
        s.events("goal_failed")
    );
    assert_eq!(said(&s, "goal_done"), "brewed 1 awkward_potion");
    assert_eq!(
        (held(&s, "awkward_potion"), held(&s, "water_potion")),
        (1, 0)
    );
    assert_eq!(
        held(&s, "blaze_powder"),
        0,
        "the powder went into the stand"
    );
    // The second goal holds no powder: the stand that stands near is taken to be fuelled.
    s.goal("craft healing_potion", true);
    assert!(
        s.run_until(900, |s| s.events("goal_done").len() == 2),
        "{:?}",
        s.events("goal_failed")
    );
    assert_eq!(
        (held(&s, "healing_potion"), held(&s, "awkward_potion")),
        (1, 0)
    );
    assert_eq!(s.world.state().stands[&STAND].charges, 18);
}

#[test]
fn three_water_bottles_go_all_the_way_to_healing_in_two_batches() {
    let mut s = sim();
    s.block(STAND, "brewing_stand");
    s.item("water_potion", 3, Some(1));
    s.item("nether_wart", 1, Some(2));
    s.item("glistering_melon_slice", 1, Some(3));
    s.item("blaze_powder", 1, Some(4));
    s.goal("craft healing_potion 3", true);
    assert!(
        s.run_until(2000, |s| s.has_event("goal_done")),
        "{:?}",
        s.events("goal_failed")
    );
    assert_eq!(said(&s, "goal_done"), "brewed 3 healing_potion");
    assert_eq!(
        (held(&s, "healing_potion"), held(&s, "nether_wart")),
        (3, 0)
    );
}

#[test]
fn an_unfuelled_stand_fails_once_naming_the_powder() {
    let mut s = sim();
    s.block(STAND, "brewing_stand");
    s.item("water_potion", 1, Some(1));
    s.item("nether_wart", 1, Some(2));
    s.goal("craft awkward_potion", true);
    assert!(s.run_until(3000, |s| s.has_event("goal_failed")));
    assert_eq!(
        said(&s, "goal_failed"),
        "cannot craft awkward_potion: need 1 blaze_powder, have 0: craft 2 from 1 blaze_rod; \
         need 1 blaze_rod, have 0. Get 1 blaze_rod."
    );
    assert_eq!(s.events("goal_failed").len(), 1);
    assert_eq!((held(&s, "water_potion"), held(&s, "nether_wart")), (1, 1));
}

#[test]
fn an_impossible_upgrade_fails_once_with_the_shopping_list() {
    let mut s = sim();
    s.block(STAND, "smithing_table");
    s.item("diamond_pickaxe", 1, Some(1));
    s.goal("craft netherite_pickaxe", true);
    assert!(s.run_until(40, |s| s.has_event("goal_failed")));
    let e = said(&s, "goal_failed");
    assert!(
        e.contains(
            "need 1 netherite_upgrade_smithing_template, have 0; need 1 netherite_ingot, \
             have 0: craft 1 from 4 netherite_scrap and 4 gold_ingot"
        ),
        "{e}"
    );
    assert!(
        e.ends_with("1 netherite_upgrade_smithing_template, 4 ancient_debris, 4 raw_gold, 1 coal."),
        "{e}"
    );
    assert_eq!(s.events("goal_failed").len(), 1);
}

#[test]
fn empty_bottles_are_filled_at_water_one_use_each() {
    let mut s = sim();
    s.block(BlockPos::new(6, 63, 0), "water");
    s.item("glass_bottle", 3, Some(4));
    s.goal("craft water_potion 3", true);
    assert!(
        s.run_until(600, |s| s.has_event("goal_done")),
        "{}",
        said(&s, "goal_failed")
    );
    assert_eq!(said(&s, "goal_done"), "filled 3 water_potion");
    assert_eq!((held(&s, "water_potion"), held(&s, "glass_bottle")), (3, 0));
}

#[test]
fn bottles_with_no_water_near_fail_once_saying_so() {
    let mut s = sim();
    s.item("glass_bottle", 3, Some(4));
    s.goal("craft water_potion 3", true);
    assert!(s.run_until(60, |s| s.has_event("goal_failed")));
    assert_eq!(
        said(&s, "goal_failed"),
        "cannot craft water_potion: no water source within 16 blocks to fill 3 glass_bottle \
         at. Go to water, or carry a water_bucket and pour it out."
    );
    assert_eq!(s.events("goal_failed").len(), 1);
    assert_eq!(held(&s, "glass_bottle"), 3);
}
