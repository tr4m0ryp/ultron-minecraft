use reflex_core::goals::kind_matches;
use reflex_core::goals::{parse, AttackTarget, GoalSpec, Person};
use reflex_core::snapshot::EntityKind;
use reflex_core::{BlockPos, Vec3};

#[test]
fn parses_every_goal_form() {
    let p = |s: &str| parse(s, None).unwrap();
    assert_eq!(
        p("mine iron_ore 3"),
        GoalSpec::Mine {
            block: "iron_ore".into(),
            count: 3
        }
    );
    assert_eq!(
        p("mine(minecraft:oak_log, 2)"),
        GoalSpec::Mine {
            block: "oak_log".into(),
            count: 2
        }
    );
    assert_eq!(
        p("collect raw_iron"),
        GoalSpec::Collect {
            item: "raw_iron".into(),
            count: 1
        }
    );
    assert_eq!(
        p("follow"),
        GoalSpec::Follow {
            who: Person::Companion,
            distance: 3.0
        }
    );
    assert_eq!(
        p("follow Alex 5"),
        GoalSpec::Follow {
            who: Person::Named("Alex".into()),
            distance: 5.0
        }
    );
    assert_eq!(
        p("go_to 1 64 -3"),
        GoalSpec::GoTo(Vec3::new(1.0, 64.0, -3.0))
    );
    assert_eq!(p("come_here"), GoalSpec::ComeHere);
    assert_eq!(p("stay"), GoalSpec::Stay);
    assert_eq!(
        p("attack zombie"),
        GoalSpec::Attack(AttackTarget::Kind(EntityKind::Zombie))
    );
    assert_eq!(p("attack #42"), GoalSpec::Attack(AttackTarget::Id(42)));
    assert_eq!(p("guard"), GoalSpec::Guard(Person::Companion));
    assert_eq!(
        p("craft stick 4"),
        GoalSpec::Craft {
            item: "stick".into(),
            count: 4
        }
    );
    assert_eq!(
        p("smelt iron_ingot 3"),
        GoalSpec::Smelt {
            item: "iron_ingot".into(),
            count: 3
        }
    );
    assert_eq!(
        p("give bread 2"),
        GoalSpec::Give {
            who: Person::Companion,
            item: "bread".into(),
            count: 2
        }
    );
    assert_eq!(
        p("give Alex bread"),
        GoalSpec::Give {
            who: Person::Named("Alex".into()),
            item: "bread".into(),
            count: 1
        }
    );
    assert_eq!(
        p("place cobblestone 2 64 -1"),
        GoalSpec::Place {
            block: "cobblestone".into(),
            pos: BlockPos::new(2, 64, -1),
            to: None,
            replace: false
        }
    );
    assert_eq!(
        p("place oak_planks 2 64 -1 6 66 -1"),
        GoalSpec::Place {
            block: "oak_planks".into(),
            pos: BlockPos::new(2, 64, -1),
            to: Some(BlockPos::new(6, 66, -1)),
            replace: false
        }
    );
    assert_eq!(
        p("place cobblestone 2 64 -1 replace"),
        GoalSpec::Place {
            block: "cobblestone".into(),
            pos: BlockPos::new(2, 64, -1),
            to: None,
            replace: true
        }
    );
    assert_eq!(
        p("place oak_planks 2 64 -1 6 66 -1 replace"),
        GoalSpec::Place {
            block: "oak_planks".into(),
            pos: BlockPos::new(2, 64, -1),
            to: Some(BlockPos::new(6, 66, -1)),
            replace: true
        }
    );
    assert_eq!(
        p("break 3 64 -2"),
        GoalSpec::Break(BlockPos::new(3, 64, -2))
    );
    assert!(parse("break", None).is_err());
    assert!(parse("place dirt 0 64 0 9 64 9", None)
        .unwrap_err()
        .contains("at most 64"));
    assert_eq!(p("sleep_in_bed"), GoalSpec::SleepInBed);
    assert_eq!(p("eat_now"), GoalSpec::EatNow);
}

#[test]
fn knob_target_fills_missing_coordinates_and_errors_are_readable() {
    let t = Some(Vec3::new(4.0, 64.0, 4.0));
    assert_eq!(
        parse("walk", t).unwrap(),
        GoalSpec::GoTo(Vec3::new(4.0, 64.0, 4.0))
    );
    assert_eq!(
        parse("", t).unwrap(),
        GoalSpec::GoTo(Vec3::new(4.0, 64.0, 4.0))
    );
    assert!(parse("dance", None).unwrap_err().contains("unknown goal"));
    assert!(parse("mine", None).unwrap_err().contains("block kind"));
    assert!(parse("mine stone lots", None)
        .unwrap_err()
        .contains("bad count"));
    assert!(parse("attack dragon", None)
        .unwrap_err()
        .contains("unknown entity"));
    assert!(parse("go_to", None).is_err());
}

#[test]
fn kind_matching_covers_variants() {
    assert!(kind_matches("bed", "red_bed"));
    assert!(kind_matches("iron_ore", "deepslate_iron_ore"));
    assert!(kind_matches("planks", "oak_planks"));
    assert!(!kind_matches("bed", "bedrock"));
    assert!(!kind_matches("ore", "iron_ores"));
}
