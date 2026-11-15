//! Flight recorder: ring wraparound and run-length rendering.

use reflex_core::trace::{Decision, GoalTick, NavCode, TickRec, TickRing};

fn rec(tick: u64, x: f32, legs: &'static str) -> TickRec {
    TickRec {
        tick,
        pos: [x, 64.0, 0.5],
        d: Decision {
            owners: [legs, "", "", "", ""],
            legs_note: "walk",
            ..Default::default()
        },
    }
}

#[test]
fn ring_wraps_and_keeps_the_newest_in_order() {
    let mut r = TickRing::with_capacity(4);
    assert!(r.is_empty());
    for t in 1..=10 {
        r.push(rec(t, 0.0, "GOAL"));
    }
    assert_eq!(r.len(), 4);
    assert_eq!(r.capacity(), 4);
    let ticks: Vec<u64> = r.iter().map(|x| x.tick).collect();
    assert_eq!(ticks, [7, 8, 9, 10]);
}

#[test]
fn partial_ring_iterates_from_the_first_push() {
    let mut r = TickRing::with_capacity(8);
    for t in 5..8 {
        r.push(rec(t, 0.0, "GOAL"));
    }
    let ticks: Vec<u64> = r.iter().map(|x| x.tick).collect();
    assert_eq!(ticks, [5, 6, 7]);
}

#[test]
fn equal_decisions_collapse_into_one_run_with_first_and_last_position() {
    let mut r = TickRing::with_capacity(32);
    for t in 0..10 {
        r.push(rec(100 + t, t as f32, "GOAL"));
    }
    for t in 10..13 {
        r.push(rec(100 + t, 9.0, "R5"));
    }
    let lines = r.render("test");
    assert_eq!(lines.len(), 3, "header plus two runs: {lines:?}");
    assert!(lines[0].contains("13 ticks, 2 runs"));
    assert!(lines[1].starts_with("TRACE t=100..109 x10 pos=0.0,64.0,0.5->9.0,64.0,0.5"));
    assert!(lines[1].contains("legs=GOAL/walk"));
    assert!(lines[2].starts_with("TRACE t=110..112 x3 pos=9.0,64.0,0.5 "));
    assert!(lines[2].contains("legs=R5"));
}

#[test]
fn a_tick_gap_breaks_a_run() {
    let mut r = TickRing::with_capacity(8);
    r.push(rec(1, 0.0, "GOAL"));
    r.push(rec(2, 0.0, "GOAL"));
    r.push(rec(9, 0.0, "GOAL"));
    assert_eq!(r.render("gap").len(), 3);
}

#[test]
fn goal_state_and_vetoes_render() {
    let mut r = TickRing::with_capacity(4);
    let mut x = rec(5, 1.0, "GOAL");
    x.d.vetoes[0] = "R5";
    x.d.goal = GoalTick {
        goal_id: 11,
        label: "go_to",
        phase: "goal_go_to",
        nav: NavCode::Moving,
        flags: 4,
        wp: [3, 71, 2],
        preempted: false,
        moving: true,
        blocked: false,
    };
    r.push(x);
    let l = r.render("why");
    assert!(l[1].contains("veto=R5"), "{}", l[1]);
    assert!(
        l[1].contains("goal=#11 go_to/goal_go_to nav=moving wp=3,71,2 sneak"),
        "{}",
        l[1]
    );
}

#[test]
fn a_long_history_is_capped() {
    let mut r = TickRing::with_capacity(400);
    for t in 0..400u64 {
        r.push(rec(t, t as f32, if t % 2 == 0 { "GOAL" } else { "R5" }));
    }
    let l = r.render("flood");
    assert!(l.len() <= 61);
    assert!(l[0].contains("omitted"));
}

/// Cost of the always-on part: `cargo test --release -p reflex-core --test trace -- --ignored
/// --nocapture`.
#[test]
#[ignore]
fn push_cost_per_tick() {
    let mut r = TickRing::default();
    let n = 2_000_000u64;
    let t0 = std::time::Instant::now();
    for t in 0..n {
        r.push(rec(t, (t % 7) as f32, "GOAL"));
    }
    let ns = t0.elapsed().as_nanos() as f64 / n as f64;
    println!(
        "ring push: {ns:.1} ns per tick (record {} bytes)",
        std::mem::size_of::<TickRec>()
    );
    let t0 = std::time::Instant::now();
    let lines = r.render("bench");
    println!(
        "render of a full ring: {} us, {} lines",
        t0.elapsed().as_micros(),
        lines.len()
    );
}

#[test]
fn a_walk_along_waypoints_is_one_run_ending_on_the_last_waypoint() {
    let mut r = TickRing::with_capacity(16);
    for t in 0..8u64 {
        let mut x = rec(10 + t, t as f32, "GOAL");
        x.d.goal = GoalTick {
            goal_id: 3,
            label: "go_to",
            nav: NavCode::Moving,
            flags: (t % 2) as u8 * 2,
            wp: [t as i32, 64, 0],
            moving: true,
            ..Default::default()
        };
        r.push(x);
    }
    let l = r.render("walk");
    assert_eq!(l.len(), 2, "{l:?}");
    assert!(
        l[1].contains("x8") && l[1].contains("wp=7,64,0"),
        "{}",
        l[1]
    );
}
