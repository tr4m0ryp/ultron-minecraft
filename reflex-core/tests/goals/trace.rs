use crate::sim::Sim;
use reflex_core::BlockPos;

fn dump(s: &mut Sim) -> Vec<String> {
    s.exec.take_trace_dump()
}

#[test]
fn a_failed_goal_dumps_the_recent_ticks() {
    let mut s = Sim::new();
    s.nav.state().unreachable.push(BlockPos::new(12, 64, 8));
    s.goal("go_to 12 64 8", true);
    let mut lines = Vec::new();
    for _ in 0..100 {
        s.step();
        lines.extend(dump(&mut s));
        if s.has_event("goal_failed") {
            break;
        }
    }
    assert!(!lines.is_empty(), "a failure leaves a dump");
    assert!(lines[0].starts_with("TRACE dump: t="), "{}", lines[0]);
    assert!(lines[0].contains("go_to failed"), "{}", lines[0]);
    assert!(
        lines.iter().any(|l| l.contains("goal=#1 go_to")),
        "{lines:?}"
    );
    assert!(lines.iter().any(|l| l.contains("nav=unreachable")));
}

#[test]
fn a_body_stuck_while_moving_dumps_before_the_navigator_gives_up() {
    let mut s = Sim::new();
    // A wall the fake planner does not know about: the body walks into it and stays put.
    for dz in 0..3 {
        s.block(BlockPos::new(2, 64, dz - 1), "stone");
        s.block(BlockPos::new(2, 65, dz - 1), "stone");
    }
    s.goal("go_to 12 64 0", true);
    let mut first = None;
    for _ in 0..140 {
        s.step();
        let l = dump(&mut s);
        if first.is_none() && !l.is_empty() {
            first = Some((s.snap.tick, l));
        }
    }
    let (tick, lines) = first.expect("a stuck walk produces a dump");
    assert!(
        tick < 1000 + 100,
        "dumped before the 100-tick path stall: {tick}"
    );
    assert!(lines[0].contains("no progress"), "{}", lines[0]);
    let last = lines.last().unwrap();
    assert!(last.contains("nav=moving") && last.contains("x"), "{last}");
}

#[test]
fn healthy_goals_print_nothing_and_the_ring_stays_readable() {
    let mut s = Sim::new();
    s.goal("go_to 12 64 8", true);
    let mut lines = Vec::new();
    for _ in 0..200 {
        s.step();
        lines.extend(dump(&mut s));
    }
    assert!(s.has_event("goal_done"));
    assert!(lines.is_empty(), "no dump on success: {lines:?}");
    let on_demand = s.exec.trace_dump("check");
    assert!(on_demand.len() < 30, "walk collapses: {}", on_demand.len());
}

/// `cargo test --release -p reflex-core --test goals -- --ignored --nocapture tick_cost`.
#[test]
#[ignore]
fn tick_cost() {
    let mut s = Sim::new();
    s.goal("go_to 12 64 8", true);
    let n = 20_000u32;
    let t0 = std::time::Instant::now();
    for _ in 0..n {
        s.step();
    }
    println!(
        "full sim tick incl. recorder: {:.1} us",
        t0.elapsed().as_nanos() as f64 / f64::from(n) / 1000.0
    );
}

#[test]
fn holding_a_spot_is_not_a_stall() {
    let mut s = Sim::new();
    s.goal("stay", true);
    for _ in 0..300 {
        s.step();
        assert!(dump(&mut s).is_empty(), "stay holds still on purpose");
    }
}

#[test]
fn scraping_a_wall_under_the_unstuck_reflex_fails_instead_of_spinning() {
    let mut s = Sim::new();
    for dz in 0..3 {
        s.block(BlockPos::new(2, 64, dz - 1), "stone");
        s.block(BlockPos::new(2, 65, dz - 1), "stone");
    }
    s.goal("go_to 12 64 0", true);
    let mut lines = Vec::new();
    assert!(
        s.run_until(1200, |s| s.has_event("goal_failed")),
        "the goal must give up, not scrape forever"
    );
    lines.extend(dump(&mut s));
    assert!(s.goal_text("goal_failed").contains("Stalled"));
    assert!(lines.iter().any(|l| l.contains("failed")), "{lines:?}");
}
