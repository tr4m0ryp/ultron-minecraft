use super::*;
use crate::snapshot::{Entity, EntityKind};

fn animal(id: u32, kind: EntityKind, x: f64) -> Entity {
    Entity::new(id, kind, Vec3::new(x, 64.0, 0.5))
}

fn me() -> Vec3 {
    Vec3::new(0.5, 64.0, 0.5)
}

#[test]
fn the_table_maps_items_to_animals_by_exact_name() {
    use EntityKind::*;
    assert_eq!(sources("wool"), Some(&[Sheep][..]));
    assert_eq!(sources("white_wool"), Some(&[Sheep][..]));
    assert_eq!(sources("mutton"), Some(&[Sheep][..]));
    assert_eq!(sources("beef"), Some(&[Cow][..]));
    assert_eq!(sources("leather"), Some(&[Cow][..]));
    assert_eq!(sources("porkchop"), Some(&[Pig][..]));
    assert_eq!(sources("feather"), Some(&[Chicken][..]));
    assert_eq!(sources("chicken"), Some(&[Chicken][..]));
    assert_eq!(sources("cooked_beef"), None, "smelted, not hunted");
    assert_eq!(sources("oak_log"), None);
}

#[test]
fn nearest_adult_sheep_is_shorn_with_shears_and_killed_without() {
    let mut baby = animal(1, EntityKind::Sheep, 2.5);
    baby.state.baby = true;
    let mut bare = animal(2, EntityKind::Sheep, 3.5);
    bare.state.sheared = true;
    let ents = vec![
        baby,
        bare,
        animal(3, EntityKind::Cow, 4.5),
        animal(4, EntityKind::Sheep, 9.5),
        animal(5, EntityKind::Sheep, 12.5),
    ];
    let mem = Memory::default();
    let take = |id, method| Choice::Take { id, method };
    assert_eq!(
        choose("white_wool", &ents, me(), true, &mem),
        take(4, Method::Shear)
    );
    assert_eq!(
        choose("wool", &ents, me(), false, &mem),
        take(4, Method::Kill)
    );
    // Mutton comes from a sheared sheep too; never from a baby.
    assert_eq!(
        choose("mutton", &ents, me(), true, &mem),
        take(2, Method::Kill)
    );
    assert_eq!(
        choose("beef", &ents, me(), true, &mem),
        take(3, Method::Kill)
    );
}

#[test]
fn memory_skips_given_up_animals_and_kills_unshearable_sheep() {
    let ents = vec![
        animal(4, EntityKind::Sheep, 9.5),
        animal(5, EntityKind::Sheep, 12.5),
    ];
    let mem = Memory {
        no_shear: vec![4],
        avoid: vec![],
    };
    let got = choose("wool", &ents, me(), true, &mem);
    assert_eq!(
        got,
        Choice::Take {
            id: 4,
            method: Method::Kill
        }
    );
    let mem = Memory {
        no_shear: vec![],
        avoid: vec![4],
    };
    let got = choose("wool", &ents, me(), true, &mem);
    assert_eq!(
        got,
        Choice::Take {
            id: 5,
            method: Method::Shear
        }
    );
}

#[test]
fn no_source_names_the_animals_and_the_reason() {
    let far = vec![animal(4, EntityKind::Sheep, SOURCE_RADIUS + 5.0)];
    match choose("white_wool", &far, me(), true, &Memory::default()) {
        Choice::NoneInRange(why) => assert_eq!(why, "no sheep within 32 blocks to hunt"),
        c => panic!("{c:?}"),
    }
    let mut bare = animal(2, EntityKind::Sheep, 3.5);
    bare.state.sheared = true;
    match choose("wool", &[bare], me(), true, &Memory::default()) {
        Choice::NoneInRange(why) => assert!(why.contains("1 sheep") && why.contains("sheared")),
        c => panic!("{c:?}"),
    }
    let mut dying = animal(3, EntityKind::Cow, 2.5);
    dying.state.dying = true;
    match choose("beef", &[dying], me(), false, &Memory::default()) {
        Choice::NoneInRange(why) => assert!(why.starts_with("no cow")),
        c => panic!("{c:?}"),
    }
    assert_eq!(
        choose("cobblestone", &[], me(), false, &Memory::default()),
        Choice::NotFromAnimals
    );
}
