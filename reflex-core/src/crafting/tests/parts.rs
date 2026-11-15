use super::{book, inv};
use crate::crafting::{choose_fuel, layout, matches, name_set};

fn cells(v: &[(usize, &str)]) -> Vec<(usize, String)> {
    v.iter().map(|(i, s)| (*i, s.to_string())).collect()
}

fn grid(items: &[Option<&str>]) -> Vec<Option<String>> {
    items.iter().map(|i| i.map(str::to_string)).collect()
}

fn recipe(id: &str) -> crate::crafting::Recipe {
    let b = book();
    let found = b.all().find(|r| r.id == id).cloned();
    found.expect(id)
}

#[test]
fn layout_picks_one_kind_per_cell_and_lowers_the_depth_for_mixed_stacks() {
    let table = recipe("crafting_table");
    let mixed = inv(&[("oak_planks", 2), ("birch_planks", 2)]);
    let l = layout(&table, 2, &mixed, 2).expect("one table fits");
    assert_eq!(l.crafts, 1);
    assert_eq!(
        l.cells,
        cells(&[
            (0, "birch_planks"),
            (1, "birch_planks"),
            (2, "oak_planks"),
            (3, "oak_planks")
        ])
    );
    let deep = layout(&table, 2, &inv(&[("oak_planks", 8)]), 2).expect("two deep");
    assert_eq!(deep.crafts, 2);
    let pick = recipe("stone_pickaxe");
    assert!(layout(&pick, 2, &inv(&[("cobblestone", 3), ("stick", 2)]), 1).is_none());
    let l = layout(&pick, 3, &inv(&[("cobblestone", 3), ("stick", 2)]), 1).expect("3x3");
    let at: Vec<usize> = l.cells.iter().map(|(i, _)| *i).collect();
    assert_eq!(at, [0, 1, 2, 4, 7], "pattern cells in a 3-wide grid");
    let planks = recipe("oak_planks");
    let l = layout(&planks, 2, &inv(&[("oak_wood", 1)]), 1).expect("shapeless");
    assert_eq!(l.cells, cells(&[(0, "oak_wood")]));
}

#[test]
fn matching_accepts_mirrored_and_shifted_patterns() {
    let axe = recipe("wooden_axe");
    let (p, s) = (Some("oak_planks"), Some("stick"));
    let straight = grid(&[p, p, None, p, s, None, None, s, None]);
    let mirrored = grid(&[None, p, p, None, s, p, None, s, None]);
    let wrong = grid(&[p, p, None, s, p, None, None, s, None]);
    assert!(matches(&axe, 3, &straight));
    assert!(matches(&axe, 3, &mirrored));
    assert!(!matches(&axe, 3, &wrong));
    let stick = recipe("stick");
    let shifted = grid(&[None, Some("birch_planks"), None, Some("oak_planks")]);
    assert!(matches(&stick, 2, &shifted));
    let torn = grid(&[Some("oak_planks"), None, None, Some("oak_planks")]);
    assert!(!matches(&stick, 2, &torn));
    let dye = recipe("dye_white_wool");
    let laid = grid(&[Some("red_wool"), None, None, Some("white_dye")]);
    assert!(matches(&dye, 2, &laid));
    assert!(!matches(
        &dye,
        2,
        &grid(&[Some("red_wool"), Some("red_wool")])
    ));
}

#[test]
fn sets_are_named_by_a_covering_tag_or_family() {
    let b = book();
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    assert_eq!(name_set(&b, &s(&["stick"])), "stick");
    assert_eq!(name_set(&b, &s(&["coal", "charcoal"])), "coal/charcoal");
    let planks = s(&[
        "oak_planks",
        "birch_planks",
        "spruce_planks",
        "bamboo_planks",
    ]);
    assert_eq!(name_set(&b, &planks), "any plank");
    let wool = s(&["red_wool", "blue_wool", "green_wool", "black_wool"]);
    assert_eq!(name_set(&b, &wool), "any wool");
    let odd = s(&["dirt", "sand", "gravel", "clay"]);
    assert_eq!(name_set(&b, &odd), "one of 4 kinds such as dirt");
}

#[test]
fn in_hand_only_marks_cycles_recolouring_and_recycling() {
    let b = book();
    for id in [
        "iron_block",
        "iron_ingot_from_iron_block",
        "iron_nugget",
        "iron_ingot_from_nuggets",
        "dye_white_wool",
        "iron_nugget_from_smelting",
    ] {
        assert!(b.in_hand_only(&recipe(id)), "{id}");
    }
    for id in [
        "stick",
        "oak_planks",
        "white_wool_from_string",
        "charcoal",
        "iron_ingot_from_smelting_raw_iron",
        "oak_wood",
    ] {
        assert!(!b.in_hand_only(&recipe(id)), "{id}");
    }
}

#[test]
fn fuel_burns_coal_first_and_counts_quarters() {
    let held = inv(&[("oak_planks", 10), ("coal", 1), ("stick", 4)]);
    assert_eq!(choose_fuel(&held, 3), (vec![("coal".to_string(), 1)], 0));
    let wood = inv(&[("oak_planks", 10), ("stick", 4)]);
    assert_eq!(
        choose_fuel(&wood, 3),
        (vec![("oak_planks".to_string(), 2)], 0)
    );
    let sticks = inv(&[("stick", 3)]);
    assert_eq!(choose_fuel(&sticks, 2), (vec![("stick".to_string(), 3)], 2));
    assert_eq!(choose_fuel(&inv(&[("crimson_planks", 9)]), 1).1, 4);
}
