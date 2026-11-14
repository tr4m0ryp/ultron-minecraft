//! Fake executor for demos and tests: a scripted toy world ticking at 20 Hz behind the
//! real bridge. Usage: brain-link-fake [socket-path] [--ticks N]

use std::time::{Duration, Instant};

use brain_link::fake::FakeWorld;
use brain_link::{default_socket_path, Bridge};

fn main() {
    let mut path = default_socket_path();
    let mut limit: Option<u64> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--ticks" => limit = args.next().and_then(|n| n.parse().ok()),
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
    let mut world = FakeWorld::scripted();
    let tick = Duration::from_millis(50);
    let mut next = Instant::now();
    while limit.is_none_or(|n| world.snap.tick < n) {
        world.step();
        let said = bridge.pump(&mut world.exec, &world.snap);
        for line in &said {
            eprintln!("brain-link-fake: chat out: {line}");
        }
        world.hear(said);
        next += tick;
        std::thread::sleep(next.saturating_duration_since(Instant::now()));
    }
}
