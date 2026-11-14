//! The staged fake world: control-file chat becomes a WAKE chat event, a fail rule fails
//! the matching goal once with its reason, and the retry finishes with goal_done.

use std::io::Write;

use brain_link::fake::{FakeWorld, Stage};
use reflex_core::events::Event;

fn run(world: &mut FakeWorld, stage: &mut Stage, ticks: u64) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..ticks {
        world.step();
        stage.tick(world);
        let now = world.snap.tick * 1000; // past the WAKE gap every tick
        out.extend(world.exec.upward.take_wake(now));
    }
    out
}

#[test]
fn chat_fail_rule_and_retry() {
    let dir = std::env::temp_dir().join(format!("stage-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ctl = dir.join("ctl");
    let mut f = std::fs::File::create(&ctl).unwrap();
    writeln!(f, "fail mine iron_ore 1 no iron_ore within 32 blocks").unwrap();
    writeln!(f, "chat hey Ultron").unwrap();
    let mut world = FakeWorld::new();
    let mut stage = Stage::new(ctl.clone());

    let woke = run(&mut world, &mut stage, 2);
    assert!(woke.iter().any(|e| e.kind == "chat" && e.text.contains("hey Ultron")));

    world.exec.knobs.set_goal("mine iron_ore 2", 0.5, None, true);
    let woke = run(&mut world, &mut stage, 60);
    let failed = woke.iter().find(|e| e.kind == "goal_failed").expect("goal_failed");
    assert!(failed.text.contains("no iron_ore within 32 blocks"));
    assert!(world.exec.knobs.goal.is_none());

    world.exec.knobs.set_goal("mine iron_ore 2", 0.5, None, true);
    let woke = run(&mut world, &mut stage, 130);
    assert!(woke.iter().any(|e| e.kind == "goal_done"));
    std::fs::remove_dir_all(&dir).ok();
}
