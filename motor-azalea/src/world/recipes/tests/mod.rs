//! Against the real jar when it is on this machine (skipped otherwise): the book loads,
//! and the planner handles the staged cases over all vanilla recipes.

mod deep;

use std::time::Instant;

use reflex_core::crafting::{plan, PlanStep, Stations};

/// (item, count, starting kit)
type Case = (&'static str, u32, &'static [(&'static str, u32)]);

fn inv(items: &[(&str, u32)]) -> Vec<(String, u32)> {
    items.iter().map(|(n, c)| (n.to_string(), *c)).collect()
}

#[test]
fn vanilla_book_plans_the_staged_cases() {
    let path = super::jar_path();
    if !path.exists() {
        eprintln!("skipped: no server jar at {}", path.display());
        return;
    }
    let (book, tags, skipped) = super::load(&path).expect("jar reads");
    assert!(
        book.len() > 1000 && tags > 200,
        "{} recipes, {tags} tags",
        book.len()
    );
    // Only kinds `parse` names as left out on purpose are skipped: nothing the grid or the
    // furnace could have made went missing by accident.
    for kind in skipped.keys() {
        let special = kind.starts_with("crafting_special_");
        let known = [
            "stonecutting",
            "blasting",
            "smoking",
            "campfire_cooking",
            "smithing_trim",
            "crafting_dye",
            // Map cloning only: its result is whatever map went in.
            "crafting_transmute",
            "crafting_decorated_pot",
            "crafting_imbue",
        ];
        assert!(special || known.contains(&kind.as_str()), "skipped {kind}");
    }
    assert!(skipped.get("crafting_transmute").is_none_or(|n| *n <= 1));
    assert!(!book.recipes_for("black_shulker_box").is_empty());
    let none = Stations::default();
    let cases: [Case; 7] = [
        ("stone_pickaxe", 1, &[("oak_log", 2), ("cobblestone", 3)]),
        (
            "iron_pickaxe",
            1,
            &[
                ("raw_iron", 3),
                ("coal", 1),
                ("oak_log", 3),
                ("cobblestone", 8),
            ],
        ),
        ("chest", 1, &[("oak_log", 3)]),
        ("oak_door", 3, &[("oak_log", 3)]),
        ("white_bed", 1, &[("white_wool", 3), ("oak_log", 2)]),
        ("bread", 2, &[("wheat", 6), ("oak_log", 1)]),
        ("torch", 4, &[("coal", 1), ("oak_log", 1)]),
    ];
    for (item, n, kit) in cases {
        let t = Instant::now();
        let p = plan(&book, &inv(kit), item, n, none, false).unwrap_or_else(|e| panic!("{e}"));
        assert!(
            t.elapsed().as_millis() < 200,
            "{item} planned in {:?}",
            t.elapsed()
        );
        let last = p.steps.last().map(PlanStep::describe).unwrap_or_default();
        assert!(last.ends_with(item), "{item}: {last}");
    }
    let e = plan(
        &book,
        &inv(&[("cobblestone", 1)]),
        "stone_pickaxe",
        1,
        none,
        false,
    );
    assert_eq!(
        e.unwrap_err(),
        "cannot craft stone_pickaxe: need 1 crafting_table, have 0: craft 1 from 4 any plank; \
         need 4 any plank, have 0: craft 4 from 1 any log; need 1 any log, have 0; \
         need 3 cobblestone/blackstone/cobbled_deepslate, have 1; \
         need 2 stick, have 0: craft 4 from 2 any plank; \
         need 2 any plank, have 0: craft 4 from 1 any log; need 1 any log, have 0. \
         Get 2 any log, 2 cobblestone/blackstone/cobbled_deepslate."
    );
    let t = Instant::now();
    for item in [
        "white_bed",
        "iron_block",
        "iron_nugget",
        "beacon",
        "red_bed",
        "cake",
    ] {
        let _ = plan(&book, &[], item, 1, none, false);
    }
    assert!(
        t.elapsed().as_millis() < 1000,
        "failing plans took {:?}",
        t.elapsed()
    );
}

#[test]
fn vanilla_book_plans_brewing_and_smithing() {
    let path = super::jar_path();
    if !path.exists() {
        eprintln!("skipped: no server jar at {}", path.display());
        return;
    }
    let (book, _, skipped) = super::load(&path).expect("jar reads");
    let at = |station: &str| book.all().filter(|r| r.station() == Some(station)).count();
    println!(
        "book {} recipes: brewing {}, smithing {}; skipped {skipped:?}",
        book.len(),
        at("brewing_stand"),
        at("smithing_table")
    );
    assert_eq!((at("brewing_stand"), at("smithing_table")), (279, 12));
    let none = Stations::default();
    let steps = |item: &str, n: u32, kit: &[(&str, u32)], near: Stations| {
        plan(&book, &inv(kit), item, n, near, false)
            .map(|p| p.steps.iter().map(PlanStep::describe).collect::<Vec<_>>())
    };
    let upgrade = [
        ("diamond_pickaxe", 1),
        ("netherite_upgrade_smithing_template", 1),
        ("netherite_ingot", 1),
        ("iron_ingot", 2),
        ("oak_log", 2),
    ];
    let s = steps("netherite_pickaxe", 1, &upgrade, none).expect("upgrade plans");
    assert_eq!(
        s.last().map(String::as_str),
        Some("smith 1 netherite_pickaxe")
    );
    assert!(s.contains(&"craft 1 smithing_table".to_string()), "{s:?}");
    let brew = [
        ("water_potion", 3),
        ("nether_wart", 1),
        ("glistering_melon_slice", 1),
        ("blaze_powder", 1),
        ("brewing_stand", 1),
    ];
    assert_eq!(
        steps("healing_potion", 3, &brew, none).expect("brew plans"),
        ["brew 3 awkward_potion", "brew 3 healing_potion"]
    );
    // The live impossible cases (tools/live/staged/goals/stations.py) hold these two lists
    // word for word; here only their ends are pinned, the middle follows the data pack.
    let stand = Stations {
        stand: true,
        stand_fuelled: true,
        smithing: true,
        table: true,
        furnace: true,
    };
    let no_upgrade = steps("netherite_pickaxe", 1, &[("diamond_pickaxe", 1)], stand).unwrap_err();
    println!("{no_upgrade}");
    assert!(no_upgrade.starts_with(
        "cannot craft netherite_pickaxe: need 1 netherite_upgrade_smithing_template, have 0"
    ));
    assert!(
        no_upgrade.contains("need 1 netherite_ingot, have 0"),
        "{no_upgrade}"
    );
    for (item, kit) in [
        (
            "netherite_pickaxe",
            vec![("diamond_pickaxe", 1), ("smithing_table", 1)],
        ),
        (
            "healing_potion",
            vec![
                ("water_potion", 1),
                ("brewing_stand", 1),
                ("blaze_powder", 1),
            ],
        ),
    ] {
        println!("LIVE {}", steps(item, 1, &kit, none).unwrap_err());
    }
    let no_potion = steps("healing_potion", 1, &[("water_potion", 1)], stand).unwrap_err();
    println!("{no_potion}");
    assert!(no_potion.starts_with(
        "cannot craft healing_potion: need 1 awkward_potion, have 0: brew 1 from 1 water_potion \
         and 1 nether_wart; need 1 nether_wart, have 0; need 1 glistering_melon_slice, have 0"
    ));
}
