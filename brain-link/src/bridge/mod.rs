//! The executor-side half: a `Send + Sync` handle the socket threads talk to, drained once
//! per tick by `pump` on the thread that owns the (non-`Send`) executor.

mod apply;
mod args;
mod chat_gate;
mod goals;
mod log;
mod observe;
mod shared;

pub use args::reflex_ids;
pub use chat_gate::{REPLY_WINDOW, UNPROMPTED_GAP};
pub use goals::{layer_spec, SUPPORTED_GOALS};

use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use reflex_core::Snapshot;
use serde_json::Value;

use crate::host::Host;
use crate::server::Server;
use crate::wire::{self, ChatLine, Events, Request, WakeEvent};
use chat_gate::ChatGate;
use shared::{Job, Shared};

/// How long a socket request waits for the tick loop before reporting it stalled.
pub const TICK_REPLY_TIMEOUT: Duration = Duration::from_secs(3);
/// Upper bound on one `wait_event` call.
pub const WAIT_MAX_S: f64 = 300.0;
pub const WAIT_DEFAULT_S: f64 = 30.0;

#[derive(Default)]
pub struct Bridge {
    shared: Mutex<Shared>,
    woke: Condvar,
    gate: Mutex<ChatGate>,
}

/// A queued request's pending reply.
pub struct Ticket(Receiver<Value>);

impl Ticket {
    pub fn wait(self, timeout: Duration) -> Value {
        self.0.recv_timeout(timeout).unwrap_or_else(|_| {
            wire::err(format!(
                "executor did not answer within {} s; the game loop is not ticking",
                timeout.as_secs()
            ))
        })
    }
}

impl Bridge {
    pub fn new() -> Arc<Bridge> {
        Arc::new(Bridge::default())
    }

    /// Creates the bridge and starts serving it on a Unix socket at `path`.
    pub fn mount(path: impl AsRef<Path>) -> std::io::Result<(Arc<Bridge>, Server)> {
        let bridge = Bridge::new();
        let server = Server::start(path.as_ref(), bridge.clone())?;
        Ok((bridge, server))
    }

    fn lock(&self) -> MutexGuard<'_, Shared> {
        self.shared.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Queues a request for the next `pump`. Requests apply in the order they were queued.
    pub fn enqueue(&self, req: Request) -> Ticket {
        let (tx, rx) = mpsc::sync_channel(1);
        self.lock().jobs.push_back(Job { req, reply: tx });
        Ticket(rx)
    }

    /// Answers one request; blocks for tick-bound requests and for `wait_event`.
    pub fn handle(&self, req: Request) -> Value {
        match req {
            Request::WaitEvent { timeout_s } => {
                let s = timeout_s.unwrap_or(WAIT_DEFAULT_S);
                if !s.is_finite() || s < 0.0 {
                    return wire::err("timeout_s must be a non-negative number");
                }
                wire::ok(self.wait_event(Duration::from_secs_f64(s.min(WAIT_MAX_S))))
            }
            Request::RecentChat { limit } => wire::ok(self.recent_chat(limit.unwrap_or(10))),
            other => self.enqueue(other).wait(TICK_REPLY_TIMEOUT),
        }
    }

    /// Returns pending WAKE events at once, or blocks until one arrives or `timeout` passes.
    pub fn wait_event(&self, timeout: Duration) -> Events {
        let deadline = Instant::now() + timeout;
        let mut s = self.lock();
        loop {
            if !s.wakes.is_empty() {
                return Events {
                    events: s.wakes.drain(..).collect(),
                    timed_out: false,
                };
            }
            let now = Instant::now();
            if now >= deadline {
                return Events {
                    events: Vec::new(),
                    timed_out: true,
                };
            }
            s = self
                .woke
                .wait_timeout(s, deadline - now)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }

    /// The last `limit` chat lines heard, oldest first.
    pub fn recent_chat(&self, limit: usize) -> Vec<ChatLine> {
        let s = self.lock();
        let skip = s.chat.len().saturating_sub(limit.min(shared::CHAT_KEEP));
        s.chat.iter().skip(skip).cloned().collect()
    }

    /// Call once per tick after `exec.tick()`: collects WAKE events and chat, applies queued
    /// knob changes in order, and answers their requests. Returns chat lines to send.
    pub fn pump<H: Host>(&self, host: &mut H, snap: &Snapshot) -> Vec<String> {
        self.pump_echo(host, snap, |_| {})
    }

    /// `pump`, also handing each new WAKE event to `echo` (the adapter logs them to stdout).
    pub fn pump_echo<H: Host>(
        &self,
        host: &mut H,
        snap: &Snapshot,
        mut echo: impl FnMut(&WakeEvent),
    ) -> Vec<String> {
        let wakes = host.upward().take_wake(snap.tick);
        let (jobs, wakes_pending) = {
            let mut s = self.lock();
            s.hear(snap);
            if !wakes.is_empty() {
                let fresh = wakes.len();
                s.push_wakes(wakes);
                for w in s.wakes.iter().rev().take(fresh).rev() {
                    echo(w);
                }
                self.woke.notify_all();
            }
            (std::mem::take(&mut s.jobs), s.wakes.len())
        };
        let mut gate = self.gate.lock().unwrap_or_else(|e| e.into_inner());
        gate.heard(&snap.chat, snap.tick);
        let mut say = Vec::new();
        for job in jobs {
            let reply = apply::run(job.req, host, snap, (&mut say, &mut gate), wakes_pending);
            let _ = job.reply.send(reply);
        }
        say
    }
}
