mod common;

use common::*;
use reflex_core::behaviours::hazard::*;
use reflex_core::channels::{MainCmd, Speed};
use reflex_core::knobs::GainName;
use reflex_core::snapshot::{BlockKind, EntityKind, ItemKind};
use reflex_core::{BlockPos, Priority, Vec3};

#[test]
fn r1_lava_withdrawal_fires_in_lava_and_not_on_stone() {
    let mut s = flat();
    let (bid, _) = eval(&mut LavaWithdrawal, &s, &knobs());
    assert!(bid.is_none());
    s.blocks.set(BlockPos::new(0, 64, 0), BlockKind::Lava);
    let bid = eval(&mut LavaWithdrawal, &s, &knobs())
        .0
        .expect("fires in lava");
    assert_eq!(bid.priority, Priority::P0);
    let legs = bid.commands.legs.unwrap();
    assert!(legs.jump && legs.heading.len() > 0.1);
}

#[test]
fn r4_contact_hazard_fire_and_magma() {
    let mut s = flat();
    assert!(eval(&mut ContactHazard, &s, &knobs()).0.is_none());
    s.blocks.set(BlockPos::new(0, 64, 0), BlockKind::Fire);
    let bid = eval(&mut ContactHazard, &s, &knobs())
        .0
        .expect("fires in fire");
    assert_eq!(bid.priority, Priority::P1);
    assert!(bid.commands.legs.unwrap().heading.len() > 0.1);

    let mut s = flat();
    s.blocks.set(BlockPos::new(0, 63, 0), BlockKind::Magma);
    assert!(eval(&mut ContactHazard, &s, &knobs()).0.is_some());
    s.me.sneaking = true;
    assert!(
        eval(&mut ContactHazard, &s, &knobs()).0.is_none(),
        "sneaking on magma is safe"
    );
}

#[test]
fn r3_extinguish_water_then_bucket_then_no_sprint() {
    let mut s = flat();
    assert!(eval(&mut Extinguish::default(), &s, &knobs()).0.is_none());
    s.me.on_fire = true;
    s.blocks.set(BlockPos::new(3, 64, 0), BlockKind::Water);
    let bid = eval(&mut Extinguish::default(), &s, &knobs()).0.unwrap();
    assert!(
        bid.commands.legs.unwrap().heading.x > 0.9,
        "steps toward the water"
    );

    s.blocks.set(BlockPos::new(3, 64, 0), BlockKind::Air);
    give(&mut s, 8, ItemKind::WaterBucket, 1);
    let mut b = Extinguish::default();
    let bid = eval(&mut b, &s, &knobs()).0.unwrap();
    assert_eq!(bid.commands.main, Some(MainCmd::SelectSlot(8)));
    s.me.selected_slot = 8;
    let bid = eval(&mut b, &s, &knobs()).0.unwrap();
    assert_eq!(bid.commands.main, Some(MainCmd::Use));

    s.me.hotbar[8] = None;
    let mut plan = walk(Vec3::new(1.0, 0.0, 0.0));
    plan.legs.as_mut().unwrap().speed = Speed::Sprint;
    assert!(veto(&mut Extinguish::default(), &s, &knobs(), &mut plan).is_some());
    assert_eq!(plan.legs.unwrap().speed, Speed::Walk);
}

#[test]
fn r5_edge_guard_vetoes_deep_drop_and_scales_with_caution() {
    let mut s = flat();
    s.me.pos = Vec3::new(1.5, 64.0, 0.5);
    let mut plan = walk(Vec3::new(1.0, 0.0, 0.0));
    assert!(
        veto(&mut EdgeGuard::default(), &s, &knobs(), &mut plan).is_none(),
        "flat ground"
    );

    cliff(&mut s, 2, Some(2));
    let mut k = knobs();
    k.set_gain(GainName::Caution, 0.0);
    let mut plan = walk(Vec3::new(1.0, 0.0, 0.0));
    assert!(
        veto(&mut EdgeGuard::default(), &s, &k, &mut plan).is_none(),
        "2-block drop ok at caution 0"
    );
    k.set_gain(GainName::Caution, 1.0);
    let v = veto(&mut EdgeGuard::default(), &s, &k, &mut plan)
        .expect("2-block drop refused at caution 1");
    assert_eq!(v.priority, Priority::P1);
    let legs = plan.legs.unwrap();
    assert_eq!(legs.speed, Speed::Sneak);
    assert!(legs.heading.len() < 1e-9);

    let mut s2 = flat();
    s2.me.pos = Vec3::new(1.5, 64.0, 0.5);
    cliff(&mut s2, 2, Some(40));
    let mut plan = walk(Vec3::new(1.0, 0.0, 0.0));
    let v = veto(&mut EdgeGuard::default(), &s2, &knobs(), &mut plan).unwrap();
    assert_eq!(
        v.priority,
        Priority::P0,
        "lethal drop escalates to the floor"
    );
}

#[test]
fn r7_void_guard_only_on_void_columns() {
    let mut s = flat();
    s.me.pos = Vec3::new(1.5, 64.0, 0.5);
    cliff(&mut s, 2, Some(10));
    let mut plan = walk(Vec3::new(1.0, 0.0, 0.0));
    assert!(veto(&mut VoidGuard::default(), &s, &knobs(), &mut plan).is_none());
    let mut s = flat();
    s.me.pos = Vec3::new(1.5, 64.0, 0.5);
    cliff(&mut s, 2, None);
    let v = veto(&mut VoidGuard::default(), &s, &knobs(), &mut plan).expect("void edge");
    assert_eq!(v.priority, Priority::P0);
}

#[test]
fn r10_dig_guard_vetoes_unsafe_digs_only() {
    let s = flat();
    let below = BlockPos::new(0, 63, 0);
    let mut plan = reflex_core::Commands::default().main(MainCmd::Dig(below));
    assert!(
        veto(&mut DigGuard::default(), &s, &knobs(), &mut plan).is_none(),
        "solid beneath"
    );
    let mut s = flat();
    s.blocks.set(BlockPos::new(0, 62, 0), BlockKind::Air);
    let v = veto(&mut DigGuard::default(), &s, &knobs(), &mut plan).unwrap();
    assert_eq!(v.priority, Priority::P1);
    assert!(plan.main.is_none());
    s.blocks.set(BlockPos::new(0, 61, 0), BlockKind::Lava);
    let mut plan = reflex_core::Commands::default().main(MainCmd::Dig(below));
    assert_eq!(
        veto(&mut DigGuard::default(), &s, &knobs(), &mut plan)
            .unwrap()
            .priority,
        Priority::P0
    );

    let mut s = flat();
    let above_head = BlockPos::new(0, 66, 0);
    s.blocks.set(above_head, BlockKind::Solid);
    s.blocks.set(above_head.above(), BlockKind::Gravity);
    let mut plan = reflex_core::Commands::default().main(MainCmd::Dig(above_head));
    assert!(
        veto(&mut DigGuard::default(), &s, &knobs(), &mut plan).is_some(),
        "gravel above"
    );
}

#[test]
fn r9_falling_block_and_suffocation() {
    let mut s = flat();
    assert!(eval(&mut FallingBlocks, &s, &knobs()).0.is_none());
    let mut sand = mob(7, EntityKind::FallingBlock, 0.5, 0.5);
    sand.pos.y = 67.0;
    s.entities.push(sand);
    assert_eq!(
        eval(&mut FallingBlocks, &s, &knobs()).0.unwrap().priority,
        Priority::P1
    );
    let mut s = flat();
    s.blocks.set(BlockPos::new(0, 65, 0), BlockKind::Solid);
    assert_eq!(
        eval(&mut FallingBlocks, &s, &knobs()).0.unwrap().priority,
        Priority::P0
    );
}

#[test]
fn r8_surface_for_air_thresholds_follow_air_floor() {
    let mut s = flat();
    s.blocks.fill(
        BlockPos::new(-3, 64, -3),
        BlockPos::new(3, 70, 3),
        BlockKind::Water,
    );
    s.blocks.fill(
        BlockPos::new(-3, 71, -3),
        BlockPos::new(3, 72, 3),
        BlockKind::Air,
    );
    s.me.head_in_water = true;
    s.me.in_water = true;
    s.me.air = 300;
    assert!(eval(&mut SurfaceForAir, &s, &knobs()).0.is_none());
    s.me.air = 50;
    let bid = eval(&mut SurfaceForAir, &s, &knobs()).0.unwrap();
    assert_eq!(bid.priority, Priority::P1);
    assert!(bid.commands.legs.unwrap().jump);
    s.me.air = 10;
    assert_eq!(
        eval(&mut SurfaceForAir, &s, &knobs()).0.unwrap().priority,
        Priority::P0
    );
    let mut k = knobs();
    k.set_gain(GainName::AirFloor, 30.0);
    s.me.air = 50;
    assert!(
        eval(&mut SurfaceForAir, &s, &k).0.is_none(),
        "brain lowered air_floor for underwater work"
    );
}
