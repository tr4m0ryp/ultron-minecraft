//! `Bridge::pump` without a socket: ordering, replies, and the real `Executor` as host.

use std::time::Duration;

use brain_link::fake::{FakeExecutor, FakeWorld};
use brain_link::wire::{self, GoalName, GoalParams, WireGain, WireMode, WireScope};
use brain_link::{Bridge, Request};
use reflex_core::knobs::Scope;
use reflex_core::Executor;

const NOW: Duration = Duration::from_millis(100);

fn go_to(x: f64) -> Request {
    Request::SetGoal {
        goal: GoalName::GoTo,
        params: GoalParams {
            pos: Some([x, 64.0, 0.5]),
            ..Default::default()
        },
        priority: Some(0.7),
    }
}

#[test]
fn pump_applies_queued_changes_in_order() {
    let bridge = Bridge::new();
    let world = FakeWorld::new();
    let mut exec = FakeExecutor::default();
    let tickets = vec![
        bridge.enqueue(Request::SetMode {
            mode: WireMode::Fight,
        }),
        bridge.enqueue(Request::SetGain {
            gain: WireGain::Aggression,
            value: 0.2,
        }),
        bridge.enqueue(go_to(10.0)),
        bridge.enqueue(go_to(20.0)),
        bridge.enqueue(Request::Inhibit {
            reflex: "r5".into(),
            scope: WireScope::UntilGoalEnds,
            strength: None,
            ttl_s: Some(5.0),
        }),
        bridge.enqueue(Request::Say { text: "one".into() }),
        bridge.enqueue(Request::Say { text: "two".into() }),
    ];
    let said = bridge.pump(&mut exec, &world.snap);
    assert_eq!(said, vec!["one", "two"]);
    let replies: Vec<_> = tickets
        .into_iter()
        .map(|t| wire::open(t.wait(NOW)))
        .collect();
    assert!(replies.iter().all(Result::is_ok), "{replies:?}");
    assert_eq!(exec.knobs.mode, reflex_core::Mode::Fight);
    assert_eq!(
        exec.knobs.gains.aggression, 0.2,
        "gain applied after the mode preset"
    );
    let active = exec.knobs.goal.as_ref().expect("goal");
    assert_eq!(active.spec, "go_to pos=20,64,0.5");
    assert_eq!(exec.knobs.goal_queue.len(), 1);
    assert_eq!(exec.knobs.goal_queue[0].spec, "go_to pos=10,64,0.5");
    let inh = &exec.knobs.inhibitions[0];
    assert_eq!(inh.behaviour, "R5");
    assert_eq!(inh.scope, Scope::UntilGoalEnds(active.id));
    assert_eq!(inh.expires_at, Some(world.snap.tick + 100));
}

#[test]
fn later_mode_resets_earlier_gain() {
    let bridge = Bridge::new();
    let world = FakeWorld::new();
    let mut exec = FakeExecutor::default();
    let a = bridge.enqueue(Request::SetGain {
        gain: WireGain::Aggression,
        value: 0.2,
    });
    let b = bridge.enqueue(Request::SetMode {
        mode: WireMode::Fight,
    });
    bridge.pump(&mut exec, &world.snap);
    assert!(wire::open(a.wait(NOW)).is_ok() && wire::open(b.wait(NOW)).is_ok());
    assert_eq!(exec.knobs.gains.aggression, 0.85);
}

#[test]
fn stalled_executor_reports_instead_of_hanging() {
    let bridge = Bridge::new();
    let reply = bridge
        .enqueue(Request::Observe)
        .wait(Duration::from_millis(200));
    let e = wire::open(reply).unwrap_err();
    assert!(e.contains("not ticking"), "{e}");
}

#[test]
fn real_executor_is_a_host() {
    let bridge = Bridge::new();
    let world = FakeWorld::new();
    let mut exec = Executor::new("Ultron");
    let mut snap = world.snap.clone();
    let t = bridge.enqueue(go_to(500.0));
    exec.tick(&snap);
    bridge.pump(&mut exec, &snap);
    assert!(wire::open(t.wait(NOW)).is_ok());
    for _ in 0..5 {
        snap.tick += 1;
        exec.tick(&snap);
        bridge.pump(&mut exec, &snap);
    }
    let obs = bridge.enqueue(Request::Observe);
    let log = bridge.enqueue(Request::Log {
        page: None,
        page_size: Some(5),
    });
    bridge.pump(&mut exec, &snap);
    let obs: wire::Observation =
        serde_json::from_value(wire::open(obs.wait(NOW)).unwrap()).unwrap();
    let goal = obs.goal.expect("goal visible");
    assert_eq!(goal.spec, "go_to pos=500,64,0.5");
    assert!(
        goal.status == "running" || goal.status.starts_with("paused_by:"),
        "{}",
        goal.status
    );
    let log: wire::LogPage = serde_json::from_value(wire::open(log.wait(NOW)).unwrap()).unwrap();
    assert!(log.lines.len() <= 5);
}
