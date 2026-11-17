//! Shopping lists for deep chains over the real recipe set (skipped without the jar): the
//! route named is the natural one, raw materials and what an ore drops, never tools to
//! melt down, an ore block to smelt or a thing to copy from itself.

use std::time::Instant;

use reflex_core::crafting::{plan, Stations};

use super::inv;

#[test]
fn vanilla_book_lists_the_natural_route_for_deep_chains() {
    let path = super::super::jar_path();
    if !path.exists() {
        eprintln!("skipped: no server jar at {}", path.display());
        return;
    }
    let (book, _, _) = super::super::load(&path).expect("jar reads");
    let all = Stations {
        table: true,
        furnace: true,
        stand: true,
        smithing: true,
        stand_fuelled: true,
    };
    // (item, kit, what the list must name, what it must not)
    type Deep = (
        &'static str,
        &'static [(&'static str, u32)],
        &'static [&'static str],
        &'static [&'static str],
    );
    let cases: [Deep; 6] = [
        (
            "glistering_melon_slice",
            &[],
            &[
                "craft 9 from 1 gold_ingot",
                "smelt 1 from 1 raw_gold",
                "Get 1 raw_gold, 1 melon_slice, 1 coal.",
            ],
            &["golden_", "kinds such as", "gold_ore"],
        ),
        (
            "healing_potion",
            &[("water_potion", 1)],
            &["Get 1 nether_wart, 1 raw_gold, 1 melon_slice, 1 coal."],
            &["golden_", "kinds such as"],
        ),
        (
            "netherite_pickaxe",
            &[("diamond_pickaxe", 1)],
            &[
                "need 1 netherite_upgrade_smithing_template, have 0; need 1 netherite_ingot",
                "smelt 4 from 4 ancient_debris",
                "smelt 4 from 4 raw_gold",
            ],
            &["7 diamond", "netherrack", "gold_ore"],
        ),
        (
            "diamond_pickaxe",
            &[("stick", 2)],
            &["need 3 diamond, have 0. Get 3 diamond."],
            &["diamond_ore", "diamond_block"],
        ),
        (
            "anvil",
            &[],
            &[
                "craft 3 from 27 iron_ingot",
                "smelt 27 from 27 raw_iron",
                "Get 31 raw_iron, 4 coal.",
            ],
            &["iron_ore", "iron_nugget", "kinds such as"],
        ),
        (
            "powered_rail",
            &[("stick", 1)],
            &["smelt 6 from 6 raw_gold", "need 1 redstone, have 0"],
            &["golden_", "redstone_ore", "redstone_block", "gold_ore"],
        ),
    ];
    for (item, kit, has, not) in cases {
        let t = Instant::now();
        let e = plan(&book, &inv(kit), item, 1, all, false).unwrap_err();
        println!("DEEP {e}");
        assert!(
            t.elapsed().as_millis() < 500,
            "{item} took {:?}",
            t.elapsed()
        );
        for h in has {
            assert!(e.contains(h), "{item} lacks {h:?}: {e}");
        }
        for n in not {
            assert!(!e.contains(n), "{item} names {n:?}: {e}");
        }
    }
}
