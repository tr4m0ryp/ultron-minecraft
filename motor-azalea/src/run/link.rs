//! The brain link mounted in the tick loop: a Unix socket the MCP server talks to. When it is
//! mounted it owns the upward channel (WAKE events go to `wait_event`, the digest to
//! `observe`) and `io::publish` does not run; WAKE lines are still echoed to stdout for logs.

use std::sync::Arc;

use brain_link::{Bridge, Host, Server};
use reflex_core::events::Upward;
use reflex_core::goals::{GoalHandle, GoalStatus};
use reflex_core::knobs::Knobs;
use reflex_core::{Executor, Snapshot};

use crate::io;

/// Set to `0` to run without the socket (the companion test bot, ablation runs).
pub const BRIDGE_ENV: &str = "ULTRON_BRIDGE";

pub struct Link {
    bridge: Arc<Bridge>,
    _server: Server,
}

impl Link {
    /// Mounts at `brain_link::default_socket_path()`; `None` when disabled or when the
    /// socket cannot be served (the loop then falls back to stdout publishing).
    pub fn mount() -> Option<Link> {
        if std::env::var(BRIDGE_ENV).is_ok_and(|v| v.trim() == "0") {
            return None;
        }
        let path = brain_link::default_socket_path();
        match Bridge::mount(&path) {
            Ok((bridge, server)) => {
                println!("BRIDGE socket={}", path.display());
                Some(Link {
                    bridge,
                    _server: server,
                })
            }
            Err(e) => {
                eprintln!("brain link not mounted at {}: {e}", path.display());
                None
            }
        }
    }
}

/// The executor plus the goal layer's status, as the bridge sees it.
struct Body<'a> {
    exec: &'a mut Executor,
    goals: &'a GoalHandle,
}

impl Host for Body<'_> {
    fn knobs(&mut self) -> &mut Knobs {
        Host::knobs(self.exec)
    }
    fn upward(&mut self) -> &mut Upward {
        Host::upward(self.exec)
    }
    fn legs_owner(&self) -> Option<String> {
        Host::legs_owner(&*self.exec)
    }
    fn goal_status(&self) -> Option<GoalStatus> {
        self.goals.status()
    }
}

/// Exactly one upward path per tick: the bridge when mounted, else stdout. Returns chat
/// lines the brain asked to say.
pub fn publish(
    link: Option<&Link>,
    exec: &mut Executor,
    goals: &GoalHandle,
    snap: &Snapshot,
) -> Vec<String> {
    let Some(link) = link else {
        io::publish(exec, snap.tick);
        return Vec::new();
    };
    let mut body = Body { exec, goals };
    link.bridge.pump_commit(
        &mut body,
        snap,
        |e| println!("WAKE t={} {} {} {}", e.t, e.kind, e.subject, e.text),
        // A box marked or lifted by a request of this tick is in the ledger file before
        // the request is answered.
        |b| sync_protected(b.goals, &mut b.exec.knobs),
    )
}

/// Mirrors the protected boxes into the ledger and writes it at once when they changed.
pub fn sync_protected(goals: &GoalHandle, knobs: &mut Knobs) {
    if goals.ledger().sync_regions(&mut knobs.protected) {
        if let Err(e) = goals.ledger().flush() {
            eprintln!("ledger not written: {e}");
        }
    }
}
