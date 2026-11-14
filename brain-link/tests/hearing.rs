//! Chat from the companion reaches `wait_event` at any distance the server delivers it,
//! through the real executor (O1 wakes) and the bridge, with and without the bot's name.

use std::time::Duration;

use brain_link::fake::FakeWorld;
use brain_link::Bridge;
use reflex_core::math::Vec3;
use reflex_core::snapshot::ChatEvent;
use reflex_core::Executor;

fn heard_after(sender: &str, text: &str, companion_at: f64) -> Vec<String> {
    let bridge = Bridge::new();
    let world = FakeWorld::new();
    let mut exec = Executor::new("Ultron");
    let mut snap = world.snap.clone();
    let me = snap.me.pos;
    let c = snap
        .companion
        .as_mut()
        .expect("the fake world has a companion");
    c.pos = me + Vec3::new(companion_at, 0.0, 0.0);
    let name = c.name.clone();
    // Far players are not in the entity list, so the line carries no sender position.
    snap.chat.push(ChatEvent {
        sender: if sender.is_empty() {
            name
        } else {
            sender.into()
        },
        text: text.into(),
        sender_pos: None,
    });
    exec.tick(&snap);
    bridge.pump(&mut exec, &snap);
    bridge
        .wait_event(Duration::from_millis(200))
        .events
        .into_iter()
        .filter(|e| e.kind == "chat")
        .map(|e| e.text)
        .collect()
}

#[test]
fn companion_line_without_the_name_wakes_from_far_away() {
    for dist in [3.0, 22.0, 90.0, 400.0] {
        let heard = heard_after("", "can you come over here?", dist);
        assert!(
            heard.iter().any(|t| t.ends_with("can you come over here?")),
            "companion at {dist} blocks: {heard:?}"
        );
    }
}

#[test]
fn a_stranger_far_away_is_heard_only_when_naming_the_bot() {
    assert!(heard_after("Alex", "anyone around?", 90.0).is_empty());
    let heard = heard_after("Alex", "ultron, over here", 90.0);
    assert_eq!(heard, vec!["<Alex> ultron, over here".to_string()]);
}
