//! The hands need a clear line: no placing and no sleeping through a wall.

use crate::sim::Sim;
use reflex_core::BlockPos;

/// A closed stone shell around `cell` (and `more`), one block thick, floor to two above.
fn shell(s: &mut Sim, cell: BlockPos, more: &[BlockPos]) {
    let inside = |p: &BlockPos| *p == cell || more.contains(p);
    for hole in std::iter::once(&cell).chain(more) {
        for dx in -1..=1 {
            for dy in 0..=1 {
                for dz in -1..=1 {
                    let p = hole.offset(dx, dy, dz);
                    if !inside(&p) {
                        s.block(p, "stone");
                    }
                }
            }
        }
    }
}

#[test]
fn a_cell_walled_in_on_every_side_is_not_placed_into() {
    // Live: the server takes a click on the floor behind a wall, and the body made it.
    let cell = BlockPos::new(3, 64, 0);
    let mut s = Sim::new();
    shell(&mut s, cell, &[]);
    s.item("cobblestone", 4, Some(13));
    s.place_at = Some(cell);
    s.goal("place cobblestone 3 64 0", true);
    assert!(s.run_until(400, |s| s.has_event("goal_failed")));
    let text = s.goal_text("goal_failed");
    assert!(text.contains("sees a face"), "{text}");
    assert_eq!(s.world.state().blocks.get(&cell), None);

    // The same cell in the open is placed from where the body stands.
    let mut s = Sim::new();
    s.item("cobblestone", 4, Some(13));
    s.place_at = Some(cell);
    s.goal("place cobblestone 3 64 0", true);
    assert!(s.run_until(200, |s| s.has_event("goal_done")));
}

#[test]
fn a_bed_behind_a_wall_is_not_slept_in() {
    let (foot, head) = (BlockPos::new(2, 64, 0), BlockPos::new(3, 64, 0));
    let mut s = Sim::new();
    s.snap.is_night = true;
    s.block(foot, "red_bed");
    s.block(head, "red_bed");
    shell(&mut s, foot, &[head]);
    // Stands right against the wall, well within the server's reach of the bed.
    s.goal("sleep_in_bed", true);
    assert!(s.run_until(200, |s| s.has_event("goal_failed")));
    let text = s.goal_text("goal_failed");
    assert!(text.contains("walled in"), "{text}");
    let w = s.world.state();
    assert!(
        !w.sleeping && w.bed_clicks == 0,
        "no click through the wall"
    );
}
