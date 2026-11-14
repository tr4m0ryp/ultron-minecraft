use crate::sim::Sim;
use reflex_core::goals::{MenuStatus, Recipe};
use reflex_core::BlockPos;

type Case = (&'static str, fn(&mut Sim), &'static str);

fn recipes(s: &mut Sim) {
    let mut w = s.world.state();
    w.recipes.insert(
        "oak_planks".into(),
        Recipe {
            ingredients: vec![("oak_log".into(), 1)],
            yields: 4,
            needs_table: false,
        },
    );
    w.recipes.insert(
        "wooden_pickaxe".into(),
        Recipe {
            ingredients: vec![("planks".into(), 3), ("stick".into(), 2)],
            yields: 1,
            needs_table: true,
        },
    );
}

#[test]
fn craft_in_the_inventory_grid() {
    let mut s = Sim::new();
    recipes(&mut s);
    s.item("oak_log", 3, None);
    s.world.state().craft_ticks = 3;
    s.goal("craft oak_planks 8", true);
    assert!(s.run_until(60, |s| s.has_event("goal_done")));
    assert_eq!(
        s.world.state().inventory,
        vec![("oak_log".to_string(), 1), ("oak_planks".to_string(), 8),]
    );
}

#[test]
fn craft_walks_to_a_table_for_three_by_three_recipes() {
    let mut s = Sim::new();
    recipes(&mut s);
    s.item("birch_planks", 3, None);
    s.item("stick", 2, None);
    s.block(BlockPos::new(10, 64, 0), "crafting_table");
    s.goal("craft wooden_pickaxe", true);
    assert!(s.run_until(200, |s| s.has_event("goal_done")));
    assert!(s.snap.me.pos.x > 6.0, "walked to the table");
}

#[test]
fn craft_reports_missing_recipe_ingredients_table_and_errors() {
    let cases: [Case; 4] = [
        ("craft beacon", |_| {}, "no known recipe"),
        (
            "craft wooden_pickaxe",
            |s| s.item("oak_planks", 3, None),
            "wooden_pickaxe: 2 more stick",
        ),
        (
            "craft wooden_pickaxe",
            |s| {
                s.item("oak_planks", 3, None);
                s.item("stick", 2, None);
            },
            "needs a crafting table",
        ),
        (
            "craft oak_planks",
            |s| {
                s.item("oak_log", 1, None);
                s.world.state().craft_error = Some("menu closed".into());
            },
            "menu closed",
        ),
    ];
    for (spec, setup, want) in cases {
        let mut s = Sim::new();
        recipes(&mut s);
        setup(&mut s);
        s.goal(spec, true);
        assert!(s.run_until(60, |s| s.has_event("goal_failed")), "{spec}");
        let text = s.goal_text("goal_failed");
        assert!(text.contains(want), "{spec}: {text}");
    }
    let mut w = Sim::new();
    assert_eq!(
        reflex_core::goals::World::craft_status(&mut w.world),
        MenuStatus::Idle
    );
}

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
