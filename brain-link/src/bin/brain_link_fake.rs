//! Fake executor for demos and tests: a scripted toy world ticking at 20 Hz behind the
//! real bridge. Usage: brain-link-fake [socket-path] [--ticks N] [--stage CONTROL_FILE]
//! With --stage the world starts quiet and is driven by lines appended to CONTROL_FILE
//! (see `fake::stage`); every chat line and goal change is logged with a wall clock.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use brain_link::fake::{stage, FakeWorld, Stage};
use brain_link::{default_socket_path, Bridge};

fn main() {
    let mut path = default_socket_path();
    let mut limit: Option<u64> = None;
    let mut control: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--ticks" => limit = args.next().and_then(|n| n.parse().ok()),
            "--stage" => control = args.next().map(PathBuf::from),
            _ => path = a.into(),
        }
    }
    let (bridge, server) = match Bridge::mount(&path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("brain-link-fake: cannot serve {}: {e}", path.display());
            std::process::exit(1);
        }
    };
    eprintln!("brain-link-fake: serving {}", server.path().display());
    let mut stage = control.map(Stage::new);
    let mut world = match stage {
        Some(_) => FakeWorld::new(),
        None => FakeWorld::scripted(),
    };
    let tick = Duration::from_millis(50);
    let mut next = Instant::now();
    while limit.is_none_or(|n| world.snap.tick < n) {
        world.step();
        if let Some(s) = stage.as_mut() {
            s.tick(&mut world);
        }
        let said = bridge.pump(&mut world.exec, &world.snap);
        for line in &said {
            match stage {
                Some(_) => stage::note(&format!("chat out: {line}")),
                None => eprintln!("brain-link-fake: chat out: {line}"),
            }
        }
        world.hear(said);
        next += tick;
        std::thread::sleep(next.saturating_duration_since(Instant::now()));
    }
}
