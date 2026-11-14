#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use brain_link::fake::FakeWorld;
use brain_link::{Bridge, Client, Server};
use reflex_core::events::EventClass;

type Job = Box<dyn FnOnce(&mut FakeWorld) + Send>;

/// A bridge on a fresh temp socket, pumped by a thread that ticks a quiet fake world every
/// 5 ms.
pub struct Rig {
    pub path: PathBuf,
    tx: Sender<Job>,
    stop: Arc<AtomicBool>,
    pump: Option<JoinHandle<()>>,
    _server: Server,
}

pub fn temp_socket() -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir()
        .join(format!("bl-test-{}-{n}", std::process::id()))
        .join("s.sock")
}

impl Rig {
    pub fn start() -> Rig {
        let path = temp_socket();
        let (bridge, server) = Bridge::mount(&path).expect("mount");
        let (tx, rx) = mpsc::channel::<Job>();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let pump = std::thread::spawn(move || {
            let mut world = FakeWorld::new();
            while !flag.load(Ordering::SeqCst) {
                while let Ok(job) = rx.try_recv() {
                    job(&mut world);
                }
                world.step();
                let said = bridge.pump(&mut world.exec, &world.snap);
                world.hear(said);
                std::thread::sleep(Duration::from_millis(5));
            }
        });
        Rig {
            path,
            tx,
            stop,
            pump: Some(pump),
            _server: server,
        }
    }

    /// Runs `f` on the tick thread between two ticks and returns its result.
    pub fn with<R: Send + 'static>(
        &self,
        f: impl FnOnce(&mut FakeWorld) -> R + Send + 'static,
    ) -> R {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send(Box::new(move |w| {
                let _ = tx.send(f(w));
            }))
            .expect("pump alive");
        rx.recv_timeout(Duration::from_secs(5))
            .expect("pump answered")
    }

    pub fn wake(&self, kind: &'static str, subject: &'static str, text: &'static str) {
        self.with(move |w| w.push(EventClass::Wake, kind, subject, text));
    }

    pub fn client(&self) -> Client {
        Client::connect(&self.path).expect("connect")
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.pump.take() {
            let _ = h.join();
        }
    }
}
