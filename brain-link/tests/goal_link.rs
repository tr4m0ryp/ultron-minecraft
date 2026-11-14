//! `set_goal` reaches every goal of the goal layer, and `observe` reports the running goal's
//! status, progress and the outcome of the last one, through the real goal runner.

use std::time::Duration;

use brain_link::bridge::layer_spec;
use brain_link::fake::FakeWorld;
use brain_link::wire::{self, GoalName, GoalParams, Observation};
use brain_link::{Bridge, Host, Request};
use reflex_core::events::Upward;
use reflex_core::goals::fakes::{FakeNav, FakeWorld as GoalWorld};
use reflex_core::goals::{self, GoalHandle, GoalStatus};
use reflex_core::knobs::Knobs;
use reflex_core::{Executor, Snapshot};

const NOW: Duration = Duration::from_millis(100);

fn params(kind: Option<&str>, pos: Option<[f64; 3]>, count: Option<u32>) -> GoalParams {
    GoalParams {
        kind: kind.map(str::to_string),
        pos,
        count,
        ..Default::default()
    }
}

#[test]
fn every_goal_maps_to_a_spec_the_goal_layer_parses() {
    let snap = FakeWorld::new().snap.clone();
    let cases = [
        (GoalName::Follow, params(None, None, None)),
        (GoalName::GoTo, params(None, Some([10.5, 64.0, -3.5]), None)),
        (GoalName::ComeHere, params(None, None, None)),
        (GoalName::Stay, params(None, None, None)),
        (GoalName::Mine, params(Some("stone"), None, Some(3))),
        (GoalName::Collect, params(Some("oak_log"), None, Some(2))),
        (GoalName::Attack, params(Some("zombie"), None, None)),
        (GoalName::Guard, params(None, None, None)),
        (GoalName::Craft, params(Some("wooden_pickaxe"), None, None)),
        (GoalName::Give, params(Some("oak_planks"), None, Some(4))),
        (
            GoalName::Place,
            params(Some("cobblestone"), Some([1.0, 64.0, 2.0]), None),
        ),
        (GoalName::SleepInBed, params(None, None, None)),
        (GoalName::EatNow, params(None, None, None)),
    ];
    for (goal, p) in cases {
        let (spec, target, _) = layer_spec(&snap, goal, &p).unwrap();
        let parsed = goals::parse(&spec, target).unwrap_or_else(|e| panic!("{spec}: {e}"));
        assert_eq!(parsed.label(), goal.label(), "{spec}");
    }
    let give = layer_spec(&snap, GoalName::Give, &params(Some("bread"), None, Some(2))).unwrap();
    assert_eq!(
        goals::parse(&give.0, None).unwrap(),
        goals::GoalSpec::Give {
            who: goals::Person::Companion,
            item: "bread".into(),
            count: 2
        }
    );
}

/// The executor plus the goal handle, as the motor adapter mounts it.
struct Body {
    exec: Executor,
    goals: GoalHandle,
}

impl Host for Body {
    fn knobs(&mut self) -> &mut Knobs {
        &mut self.exec.knobs
    }
    fn upward(&mut self) -> &mut Upward {
        &mut self.exec.upward
    }
    fn legs_owner(&self) -> Option<String> {
        Host::legs_owner(&self.exec)
    }
    fn goal_status(&self) -> Option<GoalStatus> {
        self.goals.status()
    }
}

fn body() -> Body {
    let goals = GoalHandle::default();
    let catalog = goals::catalog(
        Box::new(FakeNav::default()),
        Box::new(GoalWorld::default()),
        goals.clone(),
    );
    Body {
        exec: Executor::with_behaviours("Ultron", catalog),
        goals,
    }
}

fn tick(bridge: &Bridge, b: &mut Body, snap: &mut Snapshot) {
    snap.tick += 1;
    b.exec.tick(snap);
    b.goals.settle(&mut b.exec.knobs);
    bridge.pump(b, snap);
}

fn observe(bridge: &Bridge, b: &mut Body, snap: &mut Snapshot) -> Observation {
    let t = bridge.enqueue(Request::Observe);
    tick(bridge, b, snap);
    serde_json::from_value(wire::open(t.wait(NOW)).unwrap()).unwrap()
}

#[test]
fn observe_reports_goal_status_and_the_last_outcome() {
    let bridge = Bridge::new();
    let mut b = body();
    let mut snap = FakeWorld::new().snap.clone();
    let set = bridge.enqueue(Request::SetGoal {
        goal: GoalName::Craft,
        params: params(Some("oak_planks"), None, Some(4)),
        priority: Some(0.6),
    });
    tick(&bridge, &mut b, &mut snap);
    let ack = wire::open(set.wait(NOW)).unwrap();
    assert!(ack.to_string().contains("craft oak_planks 4"), "{ack}");
    let mut obs = observe(&bridge, &mut b, &mut snap);
    for _ in 0..40 {
        if obs.last_goal.is_some() {
            break;
        }
        obs = observe(&bridge, &mut b, &mut snap);
    }
    let last = obs.last_goal.expect("craft without logs ends");
    assert!(last.starts_with("#1 craft failed"), "{last}");
    assert!(obs.goal.is_none(), "failed goal is settled");

    let set = bridge.enqueue(Request::SetGoal {
        goal: GoalName::Stay,
        params: GoalParams::default(),
        priority: None,
    });
    tick(&bridge, &mut b, &mut snap);
    assert!(wire::open(set.wait(NOW)).is_ok());
    let obs = observe(&bridge, &mut b, &mut snap);
    let goal = obs.goal.expect("stay is active");
    assert_eq!(goal.spec, "stay");
    assert!(
        goal.status == "running" || goal.status == "arrived",
        "{}",
        goal.status
    );
}
