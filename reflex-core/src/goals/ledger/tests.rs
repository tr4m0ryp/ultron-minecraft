use super::*;

fn p(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos::new(x, y, z)
}

#[test]
fn own_cells_block_digging_exactly() {
    let l = Ledger::new();
    l.remember(p(1, 64, 1), "dirt");
    assert!(l.is_placed(p(1, 64, 1)) && l.blocks_dig(p(1, 64, 1)));
    assert!(
        !l.blocks_dig(p(2, 64, 1)),
        "no margin around the body's own cells"
    );
    assert!(l.keep().contains(p(1, 64, 1)) && !l.keep().contains(p(2, 64, 1)));
}

#[test]
fn companion_cells_shade_their_surroundings() {
    let l = Ledger::new();
    l.remember_companion(p(10, 64, 10), "cobblestone");
    assert!(l.is_placed(p(10, 64, 10)));
    assert!(
        !l.is_placed(p(11, 64, 10)),
        "placement refuses the exact cell only"
    );
    assert!(
        l.blocks_dig(p(12, 66, 8)),
        "within two blocks on every axis"
    );
    assert!(!l.blocks_dig(p(13, 64, 10)));
    assert!(l.keep().contains(p(12, 66, 8)) && !l.keep().contains(p(13, 64, 10)));
}

#[test]
fn audit_forgets_broken_or_changed_cells_and_keeps_unloaded_ones() {
    let l = Ledger::new();
    l.remember(p(0, 64, 0), "dirt");
    l.remember(p(1, 64, 0), "dirt");
    l.remember(p(2, 64, 0), "dirt");
    l.remember(p(3, 64, 0), "dirt");
    let read = |c: BlockPos| match c.x {
        0 => Some("dirt".to_string()),
        1 => Some("air".to_string()),
        2 => Some("cobblestone".to_string()),
        _ => None,
    };
    assert_eq!(l.audit(p(0, 64, 0), 8, read), 2);
    assert!(l.is_placed(p(0, 64, 0)) && l.is_placed(p(3, 64, 0)));
    assert!(!l.is_placed(p(1, 64, 0)) && !l.is_placed(p(2, 64, 0)));
    assert!(
        !l.keep().contains(p(1, 64, 0)),
        "the cache follows the change"
    );
}

#[test]
fn the_file_round_trips_and_a_missing_file_is_empty() {
    let dir = std::env::temp_dir().join(format!("ledger-test-{}", std::process::id()));
    let path = dir.join("nested").join("placed.txt");
    let l = Ledger::open(&path);
    assert!(l.is_empty());
    l.remember(p(-3, 70, 4), "dirt");
    l.remember_companion(p(5, 64, 5), "oak_planks");
    l.flush().unwrap();
    let again = Ledger::open(&path);
    assert_eq!(again.own_cells(), vec![(p(-3, 70, 4), "dirt".to_string())]);
    assert!(again.is_placed(p(5, 64, 5)) && again.blocks_dig(p(6, 65, 6)));
    again.forget(p(-3, 70, 4));
    again.flush().unwrap();
    assert!(!Ledger::open(&path).is_placed(p(-3, 70, 4)));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn protected_boxes_survive_a_restart_and_follow_the_knobs() {
    let dir = std::env::temp_dir().join(format!("ledger-regions-{}", std::process::id()));
    let path = dir.join("placed.txt");
    let base = Region::new(p(10, 60, 10), p(0, 70, 0));
    let farm = Region::new(p(20, 64, 20), p(24, 65, 24));
    let l = Ledger::open(&path);
    let mut live = vec![base];
    l.sync_regions(&mut live);
    l.remember(p(1, 64, 1), "dirt");
    l.flush().unwrap();

    let again = Ledger::open(&path);
    assert_eq!(again.regions(), vec![base]);
    assert!(
        again.is_placed(p(1, 64, 1)),
        "cells and boxes share the file"
    );
    let mut live = vec![farm];
    again.sync_regions(&mut live);
    assert_eq!(
        live,
        vec![farm, base],
        "restored boxes join one marked before"
    );
    live.retain(|r| *r != base);
    again.sync_regions(&mut live);
    again.flush().unwrap();
    assert_eq!(
        Ledger::open(&path).regions(),
        vec![farm],
        "a lifted box is gone"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// Writing in the tick a block is confirmed: the whole file is rewritten and synced, here
/// with 2000 cells in it. The cost is printed for the notes and not asserted (it is disk
/// time, 6 ms idle and several times that beside a running server).
#[test]
fn a_write_per_placed_block_keeps_every_cell() {
    let dir = std::env::temp_dir().join(format!("ultron-ledger-cost-{}", std::process::id()));
    let l = Ledger::open(dir.join("placed.txt"));
    for i in 0..2000 {
        l.remember(p(i % 50, 64 + i / 2500, i / 50), "dirt");
    }
    l.flush().expect("first write");
    let t0 = std::time::Instant::now();
    for i in 0..50 {
        l.remember(p(100 + i, 64, 0), "dirt");
        l.flush().expect("write");
    }
    let mean = t0.elapsed().as_secs_f64() * 1e3 / 50.0;
    println!("ledger write with 2000 cells: mean {mean:.2} ms");
    assert_eq!(Ledger::open(dir.join("placed.txt")).len(), 2050);
    let _ = std::fs::remove_dir_all(dir);
}
