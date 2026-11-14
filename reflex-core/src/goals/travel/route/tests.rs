use super::*;
use crate::math::{BlockPos, Vec3};

const G: i32 = 63;

fn p(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos::new(x, y, z)
}

fn near(b: BlockPos, r: f64) -> RouteGoal {
    RouteGoal::Near {
        centre: b.feet_centre() + Vec3::new(0.0, 0.5, 0.0),
        radius: r,
    }
}

fn opts(scaffold: u32) -> RouteOpts {
    RouteOpts {
        scaffold,
        ..RouteOpts::default()
    }
}

/// No hop ever digs the block its body stands on.
fn never_digs_down(r: &Route) {
    for h in &r.hops {
        assert!(!h.dig.contains(&h.from.below()), "dug under the feet: {h:?}");
    }
}

#[test]
fn open_ground_is_walked_without_work() {
    let g = Grid::new(G);
    let r = plan(&g, p(0, 64, 0), &near(p(10, 64, 0), 0.5), &opts(64));
    assert!(!r.partial);
    assert_eq!((r.digs(), r.places()), (0, 0));
    assert_eq!(r.hops.last().unwrap().to, p(10, 64, 0));
}

#[test]
fn a_walkable_detour_beats_digging() {
    let g = Grid::new(G);
    // A 2-high stone wall along x = 5 from z -6 to 6: a 14-block detour around its end.
    g.fill(p(5, 64, -6), p(5, 65, 6), Cell::STONE);
    let r = plan(&g, p(0, 64, 0), &near(p(10, 64, 0), 0.5), &opts(0));
    assert!(!r.partial);
    assert_eq!(r.digs(), 0, "walked around");
}

#[test]
fn a_thick_wall_is_dug_through_and_protection_holds() {
    let g = Grid::new(G);
    g.fill(p(5, 30, -300), p(7, 70, 300), Cell::STONE);
    let r = plan(&g, p(0, 64, 0), &near(p(10, 64, 0), 0.5), &opts(0));
    assert!(!r.partial);
    assert_eq!(r.digs(), 6, "a 1x2 tunnel through 3 blocks");
    never_digs_down(&r);
    let mut o = opts(0);
    o.protected = vec![Region::new(p(4, 20, -50), p(8, 75, 50))];
    o.max_nodes = 4_000;
    assert!(plan(&g, p(0, 64, 0), &near(p(10, 64, 0), 0.5), &o).partial);
}

#[test]
fn builds_are_never_dug() {
    let g = Grid::new(G);
    g.fill(p(5, 30, -300), p(5, 70, 300), Cell::WALL);
    let mut o = opts(0);
    o.max_nodes = 4_000;
    let r = plan(&g, p(0, 64, 0), &near(p(10, 64, 0), 0.5), &o);
    assert!(r.partial);
    assert_eq!(r.digs(), 0);
}

#[test]
fn liquid_behind_a_block_keeps_it() {
    let g = Grid::new(G);
    g.fill(p(5, 30, -300), p(5, 70, 300), Cell::STONE);
    // Water right behind the wall at both body heights: digging would let it in.
    g.fill(p(6, 30, -300), p(6, 70, 300), Cell::Water);
    let mut o = opts(0);
    o.max_nodes = 4_000;
    let r = plan(&g, p(0, 64, 0), &near(p(10, 64, 0), 0.5), &o);
    for h in &r.hops {
        assert!(h.dig.iter().all(|d| d.x != 5), "opened water: {h:?}");
    }
}

/// A sealed 2-high pocket 6 blocks underground: the body stairs its way up to the sky.
#[test]
fn a_sealed_pocket_is_escaped_by_a_staircase() {
    let g = Grid::new(G);
    g.fill(p(0, 58, 0), p(1, 59, 0), Cell::Open);
    let r = plan(&g, p(0, 58, 0), &RouteGoal::Sky { hint: p(0, 64, 0) }, &opts(0));
    assert!(!r.partial, "expanded {}", r.expanded);
    assert!(r.digs() > 0);
    never_digs_down(&r);
    let end = r.hops.last().unwrap().to;
    assert!(open_sky(&g, end) && end.y >= 64, "{end:?} {:?}", r.hops);
}

#[test]
fn a_pit_is_climbed_by_pillar_or_staircase() {
    let g = Grid::new(G);
    // A 1x1 pit 6 deep: floor at 57, feet at 58, rim at 63.
    g.fill(p(0, 58, 0), p(0, 63, 0), Cell::Open);
    let out = near(p(3, 64, 0), 0.5);
    let r = plan(&g, p(0, 58, 0), &out, &opts(16));
    assert!(!r.partial);
    assert_eq!(r.hops.iter().filter(|h| h.kind == HopKind::Pillar).count(), 5);
    assert_eq!(r.digs(), 0);
    let stairs = plan(&g, p(0, 58, 0), &out, &opts(0));
    assert!(!stairs.partial);
    assert!(stairs.digs() > 0);
    never_digs_down(&stairs);
}

#[test]
fn a_gap_is_bridged_only_with_blocks_in_hand() {
    let g = Grid::new(G);
    // A 4-wide chasm along x 3..6, 20 deep and open the whole length of the test.
    g.fill(p(3, 40, -300), p(6, 63, 300), Cell::Open);
    let goal = near(p(9, 64, 0), 0.5);
    let r = plan(&g, p(0, 64, 0), &goal, &opts(8));
    assert!(!r.partial);
    assert_eq!(r.hops.iter().filter(|h| h.kind == HopKind::Bridge).count(), 4);
    let mut o = opts(0);
    o.max_nodes = 3_000;
    o.dig = false;
    assert!(plan(&g, p(0, 64, 0), &goal, &o).partial);
}

#[test]
fn a_ledge_is_reached_by_pillaring_beside_it() {
    let g = Grid::new(G);
    // A 6-high cliff of builds (not diggable) from x 3 on.
    g.fill(p(3, 30, -300), p(12, 69, 300), Cell::WALL);
    let r = plan(&g, p(0, 64, 0), &near(p(5, 70, 0), 0.5), &opts(16));
    assert!(!r.partial);
    assert!(r.hops.iter().filter(|h| h.kind == HopKind::Pillar).count() >= 5);
    assert_eq!(r.hops.last().unwrap().to.y, 70);
}

#[test]
fn deep_stone_is_reached_by_a_staircase_from_the_side() {
    let g = Grid::new(G);
    let target = p(4, 58, 0);
    let r = plan(&g, p(0, 64, 0), &RouteGoal::Beside { block: target }, &opts(0));
    assert!(!r.partial, "{} {:?}", r.expanded, r.hops.last());
    never_digs_down(&r);
    let end = r.hops.last().unwrap().to;
    assert!(end.y == 58 || end.y == 57, "level with the block: {end:?}");
}

#[test]
fn never_pillars_under_gravel() {
    let g = Grid::new(G);
    g.set(p(0, 66, 0), Cell::STONE);
    g.set(
        p(0, 67, 0),
        Cell::Solid {
            floor: true,
            dig: Some(10),
            falls: true,
        },
    );
    g.fill(p(-1, 64, -1), p(1, 70, 1), Cell::WALL);
    g.fill(p(0, 64, 0), p(0, 65, 0), Cell::Open);
    let r = plan(&g, p(0, 64, 0), &near(p(0, 70, 0), 0.5), &opts(8));
    assert!(r.hops.iter().all(|h| h.kind != HopKind::Pillar));
}

#[test]
fn natural_terrain_is_told_from_builds() {
    for b in ["stone", "grass_block", "deepslate_iron_ore", "oak_leaves", "gravel"] {
        assert!(natural(b), "{b}");
    }
    for b in ["cobblestone", "oak_planks", "oak_log", "glass", "infested_stone", "ice"] {
        assert!(!natural(b), "{b}");
    }
    assert_eq!(drop_of("stone"), "cobblestone");
    assert_eq!(drop_of("oak_log"), "oak_log");
}
