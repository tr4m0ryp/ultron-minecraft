//! `collect` taking items from animals: shearing, killing, and precise failures.

use crate::sim::Sim;
use reflex_core::snapshot::{EntityKind, ItemKind};

fn sheep_sim() -> Sim {
    let mut s = Sim::new();
    s.loot = vec![
        (EntityKind::Sheep, "white_wool", 1),
        (EntityKind::Sheep, "mutton", 2),
        (EntityKind::Cow, "beef", 2),
        (EntityKind::Cow, "leather", 1),
    ];
    s
}

fn count(s: &Sim, item: &str) -> u32 {
    s.world
        .state()
        .inventory
        .iter()
        .filter(|(k, _)| k == item)
        .map(|(_, n)| *n)
        .sum()
}

#[test]
fn collect_wool_kills_a_sheep_without_shears_and_picks_up_the_drop() {
    let mut s = sheep_sim();
    s.hotbar(0, ItemKind::Sword, 1);
    s.mob(21, EntityKind::Sheep, 9.5, 3.5);
    s.goal("collect white_wool 1", true);
    assert!(
        s.run_until(600, |s| s.has_event("goal_done")),
        "{:?}",
        s.events("goal_failed")
    );
    assert!(s.attacked.contains(&21));
    assert!(s.interacted.is_empty());
    assert_eq!(count(&s, "white_wool"), 1);
    assert!(s.goal_text("goal_done").contains("killed 1"));
}

#[test]
fn collect_wool_shears_the_sheep_when_shears_are_at_hand() {
    let mut s = sheep_sim();
    s.hotbar(0, ItemKind::Sword, 1);
    s.hotbar(3, ItemKind::Other, 1);
    s.item("shears", 1, Some(3));
    s.mob(21, EntityKind::Sheep, 9.5, 3.5);
    s.goal("collect wool 2", true);
    assert!(
        s.run_until(600, |s| s.has_event("goal_done")),
        "{:?}",
        s.events("goal_failed")
    );
    assert!(s.attacked.is_empty(), "the sheep lives");
    assert!(s.interacted.contains(&21));
    assert!(s.snap.entity(21).is_some_and(|e| e.state.sheared));
    assert_eq!(count(&s, "white_wool"), 2);
    assert!(s.goal_text("goal_done").contains("sheared 1"));
}

#[test]
fn shearing_that_does_nothing_falls_back_to_the_kill() {
    let mut s = sheep_sim();
    s.shears_work = false;
    s.hotbar(3, ItemKind::Other, 1);
    s.item("shears", 1, Some(3));
    s.mob(21, EntityKind::Sheep, 5.5, 0.5);
    s.goal("collect wool 1", true);
    assert!(
        s.run_until(800, |s| s.has_event("goal_done")),
        "{:?}",
        s.events("goal_failed")
    );
    assert!(s.interacted.len() >= 4);
    assert!(s.attacked.contains(&21));
    assert!(s
        .goal_text("goal_blocked")
        .contains("shearing the sheep did nothing"));
}

#[test]
fn collect_beef_hunts_cows_until_the_count_is_met() {
    let mut s = sheep_sim();
    s.hotbar(0, ItemKind::Sword, 1);
    let mut calf = reflex_core::snapshot::Entity::new(
        30,
        EntityKind::Cow,
        reflex_core::Vec3::new(3.5, 64.0, 0.5),
    );
    calf.state.baby = true;
    s.snap.entities.push(calf);
    s.mob(31, EntityKind::Cow, 8.5, -4.5);
    s.mob(32, EntityKind::Cow, -9.5, 6.5);
    s.mob(33, EntityKind::Sheep, 4.5, 4.5);
    s.goal("collect beef 3", true);
    assert!(
        s.run_until(1500, |s| s.has_event("goal_done")),
        "{:?}",
        s.events("goal_failed")
    );
    assert!(count(&s, "beef") >= 3);
    assert!(s.attacked.contains(&31) && s.attacked.contains(&32));
    assert!(!s.attacked.contains(&30), "calves drop nothing");
    assert!(!s.attacked.contains(&33), "not a cow");
}

#[test]
fn no_source_animal_fails_with_the_precise_reason() {
    let mut s = sheep_sim();
    s.mob(31, EntityKind::Cow, 8.5, -4.5);
    s.goal("collect white_wool 3", true);
    assert!(s.run_until(100, |s| s.has_event("goal_failed")));
    assert!(s.goal_text("goal_failed").ends_with(
        "collect: no white_wool drops within 16 blocks and no sheep within 32 blocks to hunt; \
         collected 0 of 3"
    ));
    assert!(s.attacked.is_empty());

    let mut s = sheep_sim();
    s.goal("collect cobblestone 3", true);
    assert!(s.run_until(100, |s| s.has_event("goal_failed")));
    assert!(s
        .goal_text("goal_failed")
        .contains("no cobblestone drops within 16 blocks;"));
}

#[test]
fn wool_already_on_the_ground_is_picked_up_before_any_hunt() {
    let mut s = sheep_sim();
    s.mob(21, EntityKind::Sheep, -9.5, 0.5);
    s.world.state().drops.push(reflex_core::goals::ItemDrop {
        entity_id: 600,
        item: "white_wool".into(),
        count: 1,
        pos: reflex_core::Vec3::new(4.5, 64.0, 0.5),
    });
    s.goal("collect wool 1", true);
    assert!(s.run_until(200, |s| s.has_event("goal_done")));
    assert!(s.attacked.is_empty());
}

#[test]
fn attack_names_a_species_and_reports_what_the_kill_picked_up() {
    let mut s = sheep_sim();
    s.hotbar(0, ItemKind::Sword, 1);
    s.mob(31, EntityKind::Cow, 3.5, 0.5);
    s.mob(21, EntityKind::Sheep, 6.5, 0.5);
    s.goal("attack sheep", true);
    assert!(s.run_until(600, |s| s.has_event("goal_done")));
    assert!(s.attacked.contains(&21) && !s.attacked.contains(&31));

    // Killed at the feet, the drops land in the inventory during the fight (live: the wool
    // was always picked up before a following `collect` began, which then counted 0).
    let mut s = sheep_sim();
    s.hotbar(0, ItemKind::Sword, 1);
    s.mob(21, EntityKind::Sheep, 4.5, 0.5);
    s.goal("attack sheep", true);
    s.run_until(5, |_| false);
    s.item("white_wool", 1, None);
    s.item("mutton", 2, None);
    assert!(s.run_until(600, |s| s.has_event("goal_done")));
    let text = s.goal_text("goal_done");
    assert!(text.contains("picked up 3 items"), "{text}");

    // `passive` still names every animal.
    let mut s = sheep_sim();
    s.mob(31, EntityKind::Cow, 3.5, 0.5);
    s.goal("attack passive", true);
    assert!(s.run_until(600, |s| s.has_event("goal_done")));
    assert!(s.attacked.contains(&31));
}
