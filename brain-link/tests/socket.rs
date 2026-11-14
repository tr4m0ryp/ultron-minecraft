//! Every request over a real temp socket against the fake executor.

mod common;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

use brain_link::wire::{self, Ack, ChatLine, Events, LogPage, Observation};
use common::Rig;
use reflex_core::knobs::{Response, Scope, Trigger};
use reflex_core::snapshot::EntityKind;
use serde_json::{json, Value};

fn call(c: &mut brain_link::Client, req: Value) -> Result<Value, String> {
    wire::open(c.raw(&req).expect("transport"))
}

fn ack(c: &mut brain_link::Client, req: Value) -> Ack {
    serde_json::from_value(call(c, req).expect("ok")).unwrap()
}

#[test]
fn every_request_round_trips() {
    let rig = Rig::start();
    let mut c = rig.client();

    let o: Observation =
        serde_json::from_value(call(&mut c, json!({"op":"observe"})).unwrap()).unwrap();
    assert_eq!(o.held.as_deref(), Some("sword"));
    assert_eq!(o.companion.as_ref().unwrap().name, "Steve");
    assert_eq!(o.companion.as_ref().unwrap().dist, 3.0);
    assert!(o.goal.is_none() && o.near.iter().any(|g| g.kind == "passive" && g.n == 2));

    let a = ack(
        &mut c,
        json!({"op":"set_goal","goal":"go_to","params":{"pos":[5000,64,0.5]},"priority":0.8}),
    );
    let id = a.id.unwrap();
    let o: Observation =
        serde_json::from_value(call(&mut c, json!({"op":"observe"})).unwrap()).unwrap();
    let g = o.goal.unwrap();
    assert_eq!((g.id, g.status.as_str(), g.priority), (id, "running", 0.8));

    ack(
        &mut c,
        json!({"op":"set_goal","goal":"follow","params":{"distance":5}}),
    );
    ack(&mut c, json!({"op":"set_mode","mode":"build"}));
    ack(
        &mut c,
        json!({"op":"set_gain","gain":"vigilance","value":0.9}),
    );
    let (mode, leash, vig, goal) = rig.with(|w| {
        let k = &w.exec.knobs;
        (
            k.mode,
            k.gains.leash,
            k.gains.vigilance,
            k.goal.clone().unwrap().spec,
        )
    });
    assert_eq!((mode, leash, vig), (reflex_core::Mode::Build, 6.0, 0.9));
    assert_eq!(goal, "follow distance=5");

    ack(
        &mut c,
        json!({"op":"inhibit","reflex":"R5","scope":{"kind":"here","radius":4},"ttl_s":10}),
    );
    let inh = rig.with(|w| w.exec.knobs.inhibitions.clone());
    assert!(matches!(inh[0].scope, Scope::Region(_)) && inh[0].strength == 1.0);

    ack(
        &mut c,
        json!({"op":"arm","trigger":{"kind":"entity_within","entity":"creeper","dist":6,"of_companion":true},
        "response":{"kind":"place_block_between","entity":"creeper"},"ttl_s":300}),
    );
    let armed = rig.with(|w| w.exec.knobs.armed.clone());
    assert_eq!(
        armed[0].trigger,
        Trigger::EntityWithin {
            kind: EntityKind::Creeper,
            dist: 6.0,
            of_companion: true
        }
    );
    assert_eq!(
        armed[0].response,
        Response::PlaceBlockBetween(EntityKind::Creeper)
    );
    assert!(armed[0].once);

    ack(&mut c, json!({"op":"say","text":"on my way"}));
    let deadline = Instant::now() + Duration::from_secs(2);
    let heard = loop {
        let chat: Vec<ChatLine> =
            serde_json::from_value(call(&mut c, json!({"op":"recent_chat","limit":5})).unwrap())
                .unwrap();
        if !chat.is_empty() || Instant::now() > deadline {
            break chat;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(
        (heard[0].from.as_str(), heard[0].text.as_str()),
        ("Ultron", "on my way")
    );

    rig.wake("chat", "Steve", "<Steve> hi");
    let e: Events =
        serde_json::from_value(call(&mut c, json!({"op":"wait_event","timeout_s":2})).unwrap())
            .unwrap();
    assert_eq!(e.events[0].kind, "chat");

    let p: LogPage =
        serde_json::from_value(call(&mut c, json!({"op":"log","page_size":3})).unwrap()).unwrap();
    assert!(p.total >= 1 && p.lines.len() <= 3 && p.lines[0].class == 'W');
}

#[test]
fn wait_event_blocks_then_wakes_and_times_out() {
    let rig = Rig::start();
    let mut c = rig.client();
    let t0 = Instant::now();
    let e: Events =
        serde_json::from_value(call(&mut c, json!({"op":"wait_event","timeout_s":0.3})).unwrap())
            .unwrap();
    assert!(e.timed_out && e.events.is_empty() && t0.elapsed() >= Duration::from_millis(300));

    let path = rig.path.clone();
    let waiter = std::thread::spawn(move || {
        let mut c = brain_link::Client::connect(&path).unwrap();
        let t0 = Instant::now();
        let v = call(&mut c, json!({"op":"wait_event","timeout_s":20})).unwrap();
        (t0.elapsed(), serde_json::from_value::<Events>(v).unwrap())
    });
    std::thread::sleep(Duration::from_millis(400));
    rig.wake("goal_reached", "", "go_to pos=1,2,3");
    let (took, e) = waiter.join().unwrap();
    assert!(!e.timed_out && e.events[0].kind == "goal_reached");
    assert!(
        took >= Duration::from_millis(350) && took < Duration::from_secs(3),
        "{took:?}"
    );
}

#[test]
fn malformed_input_gets_an_error_and_the_server_lives() {
    let rig = Rig::start();
    let s = UnixStream::connect(&rig.path).unwrap();
    let mut out = s.try_clone().unwrap();
    let mut inp = BufReader::new(s);
    let bad = [
        "this is not json",
        r#"{"op":"fly"}"#,
        r#"{"op":"set_goal","goal":"dance"}"#,
        r#"{"op":"set_gain","gain":"caution","value":0.1,"extra":1}"#,
        r#"{"op":"set_goal","goal":"go_to"}"#,
        r#"{"op":"set_goal","goal":"mine","params":{"kind":"iron_ore"}}"#,
        r#"{"op":"inhibit","reflex":"R99"}"#,
        r#"{"op":"arm","trigger":{"kind":"entity_within","entity":"dragon","dist":4},"response":{"kind":"say","text":"x"}}"#,
        r#"{"op":"say","text":""}"#,
    ];
    let mut errors = Vec::new();
    for line in bad {
        writeln!(out, "{line}").unwrap();
        let mut reply = String::new();
        inp.read_line(&mut reply).unwrap();
        errors.push(wire::open(serde_json::from_str(&reply).unwrap()).unwrap_err());
    }
    assert!(
        errors[2].contains("unknown variant `dance`"),
        "{}",
        errors[2]
    );
    assert!(errors[4].contains("needs param 'pos'"), "{}", errors[4]);
    assert!(
        errors[5].contains("not supported by this executor yet"),
        "{}",
        errors[5]
    );
    assert!(errors[6].contains("unknown reflex"), "{}", errors[6]);
    assert!(errors[7].contains("unknown entity kind"), "{}", errors[7]);
    writeln!(out, r#"{{"op":"observe"}}"#).unwrap();
    let mut reply = String::new();
    inp.read_line(&mut reply).unwrap();
    assert!(wire::open(serde_json::from_str(&reply).unwrap()).is_ok());
    assert!(call(&mut rig.client(), json!({"op":"observe"})).is_ok());
}

#[test]
fn two_clients_while_one_waits() {
    let rig = Rig::start();
    let path = rig.path.clone();
    let waiter = std::thread::spawn(move || {
        let mut c = brain_link::Client::connect(&path).unwrap();
        call(&mut c, json!({"op":"wait_event","timeout_s":20})).unwrap()
    });
    std::thread::sleep(Duration::from_millis(100));
    let mut b = rig.client();
    assert!(call(&mut b, json!({"op":"observe"})).is_ok());
    ack(&mut b, json!({"op":"set_goal","goal":"stay"}));
    rig.wake("companion_far", "Steve", "Steve is 40 blocks away");
    let e: Events = serde_json::from_value(waiter.join().unwrap()).unwrap();
    assert_eq!(e.events[0].subject, "Steve");
}

#[test]
fn second_executor_cannot_steal_a_live_socket() {
    let rig = Rig::start();
    let err = brain_link::Bridge::mount(&rig.path).err().expect("refused");
    assert_eq!(err.kind(), std::io::ErrorKind::AddrInUse);
}
