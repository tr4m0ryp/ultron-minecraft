use std::path::PathBuf;
use std::time::Duration;

use reflex_core::events::{Event, EventClass};
use reflex_core::goals::{GoalState, GoalStatus};
use reflex_core::math::Vec3;
use reflex_core::snapshot::{Entity, EntityKind};
use serde_json::{json, Value};

use super::{Record, FORMAT};
use crate::bridge::Bridge;
use crate::fake::FakeWorld;
use crate::wire::{GoalName, GoalParams, Request, WireMode};

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ultron-decisions-{}", std::process::id()));
    let p = dir.join(format!("{name}.jsonl"));
    let _ = std::fs::remove_file(&p);
    p
}

fn lines(p: &PathBuf) -> Vec<Value> {
    std::fs::read_to_string(p)
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str(l).expect("one JSON value per line"))
        .collect()
}

/// A world in the middle of a mine goal with one goal queued, items held and events heard.
fn busy_world() -> FakeWorld {
    let mut w = FakeWorld::new();
    w.snap.tick = 4321;
    w.snap.day_time = Some(13_000);
    w.snap.is_night = true;
    w.snap.me.health = 17.5;
    w.snap.me.pos = Vec3::new(10.5, 64.0, -3.25);
    w.snap
        .entities
        .push(Entity::new(9, EntityKind::Zombie, Vec3::new(14.5, 64.0, -3.25)));
    w.exec.inventory = vec![
        ("cobblestone".into(), 40),
        ("oak_log".into(), 3),
        ("cobblestone".into(), 9),
    ];
    w.exec.knobs.set_goal("collect oak_log 4", 0.5, None, true);
    let id = w.exec.knobs.set_goal("mine stone 8", 0.6, None, true);
    w.exec.status = Some(GoalStatus {
        goal_id: id,
        label: "mine",
        state: GoalState::Running,
        progress: Some((5, 8)),
        note: Some("mined 5 stone".into()),
    });
    w.exec.legs_owner = Some("GOAL".into());
    w.exec
        .upward
        .push(Event::new(4300, EventClass::Digest, "goal_progress", "mine").text("mined 4 stone"));
    w.exec
        .upward
        .push(Event::new(4310, EventClass::Log, "suppressed", "R5"));
    w
}

fn decide(bridge: &Bridge, w: &mut FakeWorld, req: Request) -> Value {
    let t = bridge.enqueue(req);
    bridge.pump(&mut w.exec, &w.snap);
    t.wait(Duration::from_millis(200))
}

#[test]
fn a_set_goal_is_recorded_with_this_ticks_state_and_every_parameter() {
    let path = temp("format");
    let bridge = Bridge::new();
    bridge.record_decisions(&path).expect("log opens");
    let mut w = busy_world();
    let call = Request::SetGoal {
        goal: GoalName::Place,
        params: GoalParams {
            kind: Some("cobblestone".into()),
            pos: Some([10.0, 64.0, -5.0]),
            to: Some([12.0, 66.0, -5.0]),
            replace: Some(true),
            ..Default::default()
        },
        priority: Some(0.7),
    };
    let reply = decide(&bridge, &mut w, call.clone());
    assert_eq!(reply["ok"], true, "{reply}");
    let got = lines(&path);
    assert_eq!(got.len(), 1);
    let r = &got[0];
    assert_eq!(r["v"], FORMAT);
    assert_eq!(r["tick"], 4321);
    assert!(r["wall_ms"].as_u64().is_some_and(|ms| ms > 1_600_000_000_000));
    assert_eq!(
        r["call"],
        json!({"op": "set_goal", "goal": "place", "priority": 0.7, "params": {
            "kind": "cobblestone", "pos": [10.0, 64.0, -5.0], "to": [12.0, 66.0, -5.0],
            "replace": true}})
    );
    assert_eq!(r["result"]["ok"], true);
    assert_eq!(r["result"]["id"], 3);
    let s = &r["state"];
    assert_eq!((&s["hp"], &s["food"]), (&json!(17.5), &json!(18)));
    assert_eq!(s["pos"], json!([10.5, 64.0, -3.25]));
    assert_eq!((&s["day_time"], &s["night"]), (&json!(13_000), &json!(true)));
    assert_eq!((&s["mode"], &s["legs"]), (&json!("explore"), &json!("GOAL")));
    assert_eq!(s["inventory"], json!({"cobblestone": 49, "oak_log": 3}));
    assert_eq!(s["companion"]["name"], "Steve");
    assert_eq!(s["entities"].as_array().map(Vec::len), Some(4));
    assert_eq!(
        s["entities"][1],
        json!({"kind": "zombie", "id": 9, "dist": 4.0, "pos": [14.5, 64.0, -3.25]})
    );
    // The state is the one the decision was made in: the old goal still runs.
    assert_eq!(
        s["goals"]["running"],
        json!({"id": 2, "spec": "mine stone 8", "priority": 0.6, "state": "running",
            "progress": [5, 8], "note": "mined 5 stone"})
    );
    assert_eq!(
        s["goals"]["queued"],
        json!([{"id": 1, "spec": "collect oak_log 4", "priority": 0.5}])
    );
    assert_eq!(
        s["events"],
        json!([{"t": 4300, "class": "D", "kind": "goal_progress", "subject": "mine",
            "text": "mined 4 stone"}])
    );
    let back: Record = serde_json::from_value(r.clone()).expect("a record reads back");
    assert_eq!(back.call, call);
    assert_eq!(back.state.inventory["cobblestone"], 49);
}

#[test]
fn every_decision_is_logged_once_and_reads_are_not() {
    let path = temp("ops");
    let bridge = Bridge::new();
    bridge.record_decisions(&path).expect("log opens");
    let mut w = busy_world();
    let parse = |v: Value| serde_json::from_value::<Request>(v).expect("request parses");
    let calls = [
        json!({"op": "observe"}),
        json!({"op": "set_mode", "mode": "mine"}),
        json!({"op": "set_gain", "gain": "caution", "value": 0.8}),
        json!({"op": "inhibit", "reflex": "R5", "scope": {"kind": "here", "radius": 6},
            "ttl_s": 30}),
        json!({"op": "arm", "trigger": {"kind": "chat_matches", "pattern": "ping"},
            "response": {"kind": "say", "text": "pong"}}),
        json!({"op": "say", "text": "digging here"}),
        json!({"op": "log"}),
        json!({"op": "set_goal", "goal": "go_to"}),
        json!({"op": "set_goal", "goal": "end"}),
    ];
    for c in &calls {
        w.snap.tick += 1;
        decide(&bridge, &mut w, parse(c.clone()));
    }
    let got = lines(&path);
    let ops: Vec<&str> = got.iter().filter_map(|r| r["call"]["op"].as_str()).collect();
    assert_eq!(
        ops,
        ["set_mode", "set_gain", "inhibit", "arm", "say", "set_goal", "set_goal"]
    );
    assert_eq!(got[2]["call"]["scope"], json!({"kind": "here", "radius": 6.0}));
    assert_eq!(got[2]["call"]["ttl_s"], 30.0);
    // A refused call is a decision too, recorded with the reason.
    assert_eq!(got[5]["result"]["ok"], false);
    assert!(got[5]["result"]["text"].as_str().is_some_and(|t| t.contains("pos")));
    assert_eq!(got[1]["state"]["mode"], "mine", "the mode set one call earlier");
    let ticks: Vec<u64> = got.iter().filter_map(|r| r["tick"].as_u64()).collect();
    assert_eq!(ticks, [4323, 4324, 4325, 4326, 4327, 4329, 4330]);
    assert_eq!(bridge.decision_cost().records, 7);
}

#[test]
fn no_file_is_written_until_the_log_is_started() {
    let bridge = Bridge::new();
    let mut w = busy_world();
    decide(&bridge, &mut w, Request::SetMode { mode: WireMode::Mine });
    assert_eq!(bridge.decision_cost().records, 0);
}

/// The cost on the tick thread, printed for the notes: a full record (24 entities, 36 item
/// kinds, 12 events) built and written 500 times.
#[test]
fn a_full_record_costs_far_less_than_a_tick() {
    let path = temp("cost");
    let bridge = Bridge::new();
    bridge.record_decisions(&path).expect("log opens");
    let mut w = busy_world();
    for i in 0..40u32 {
        let at = Vec3::new(f64::from(i), 64.0, 2.0);
        w.snap.entities.push(Entity::new(100 + i, EntityKind::Zombie, at));
        w.exec.inventory.push((format!("item_{i}"), i + 1));
        let e = Event::new(4000 + u64::from(i), EventClass::Digest, "unstuck", "").text("x");
        w.exec.upward.push(e);
    }
    for _ in 0..500 {
        w.snap.tick += 1;
        decide(&bridge, &mut w, Request::SetMode { mode: WireMode::Mine });
    }
    let c = bridge.decision_cost();
    let mean = c.total.as_secs_f64() * 1e6 / c.records as f64;
    let bytes = std::fs::metadata(&path).map_or(0, |m| m.len()) / 500;
    println!(
        "decision log: mean {mean:.1} us, max {} us, {bytes} bytes per record",
        c.max.as_micros()
    );
    assert_eq!(c.records, 500);
    assert!(mean < 2000.0, "mean {mean} us");
}
