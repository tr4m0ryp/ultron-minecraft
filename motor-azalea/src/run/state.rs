use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

use azalea::prelude::*;
use reflex_core::Executor;

use crate::{
    act::{self, HandState},
    io::{self, Inbox, Trace},
    sense::{self, EventBuffer, SenseState},
};

/// What lives on the handler thread only.
struct Local {
    exec: Executor,
    inbox: Inbox,
    trace: Trace,
    goals: reflex_core::goals::GoalHandle,
}

thread_local! {
    // `Executor` holds boxed behaviours that are not `Send`, so it cannot live in the ECS
    // component. Azalea runs client handlers on one local task set, which is one thread.
    static LOCAL: RefCell<Option<Local>> = const { RefCell::new(None) };
}

#[derive(Default)]
struct Inner {
    sense: SenseState,
    events: EventBuffer,
    hands: HandState,
}

/// Per-client state stored as an ECS component, cloned cheaply into each handler call.
#[derive(Clone, Component, Default)]
pub struct State {
    inner: Arc<Mutex<Inner>>,
    companion: Option<Arc<str>>,
}

impl State {
    /// `companion` is the username of the human player the bot plays alongside.
    pub fn new(companion: Option<String>) -> Self {
        Self {
            inner: Arc::default(),
            companion: companion.map(Into::into),
        }
    }

    fn tick(&self, bot: &Client) {
        let started = std::time::Instant::now();
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let inner = &mut *guard;
        let Some(snap) = sense::snapshot(
            bot,
            &mut inner.sense,
            &mut inner.events,
            self.companion.as_deref(),
        ) else {
            return;
        };
        let sensed = started.elapsed();
        let (out, say) = LOCAL.with_borrow_mut(|slot| {
<<<<<<< HEAD
            let l = slot.get_or_insert_with(|| Local {
                exec: executor(&bot.username()),
                inbox: Inbox::spawn_stdin(),
                trace: Trace::default(),
=======
            let l = slot.get_or_insert_with(|| {
                let (behaviours, goals) = crate::goal_catalog(bot);
                Local {
                    exec: Executor::with_behaviours(&bot.username(), behaviours),
                    inbox: Inbox::spawn_stdin(),
                    trace: Trace::default(),
                    goals,
                }
>>>>>>> refs/heads/goals-nav
            });
            let say = l.inbox.drain(&mut l.exec);
            let out = l.exec.tick(&snap);
            l.goals.settle(&mut l.exec.knobs);
            l.trace.tick(&snap, &l.exec, sensed);
            io::publish(&mut l.exec, snap.tick);
            (out, say)
        });
        act::apply(bot, &out, &snap.me, &mut inner.hands);
        for text in say {
            bot.chat(text.as_str());
        }
    }
}

/// The full catalog, or only the behaviours named in `MOTOR_ONLY` (comma-separated ids such
/// as `D2,D11`), which live ablation runs use as a crude baseline.
fn executor(name: &str) -> Executor {
    let Ok(only) = std::env::var("MOTOR_ONLY") else {
        return Executor::new(name);
    };
    let keep: Vec<&str> = only.split(',').map(str::trim).collect();
    let catalog = reflex_core::behaviours::default_catalog()
        .into_iter()
        .filter(|b| keep.contains(&b.id()))
        .collect();
    Executor::with_behaviours(name, catalog)
}

pub async fn handle(bot: Client, event: Event, state: State) {
    match event {
        Event::Tick => state.tick(&bot),
        Event::Disconnect(reason) => {
            eprintln!("disconnected: {reason:?}");
            bot.exit();
        }
        Event::ConnectionFailed(err) => {
            eprintln!("connection failed: {err}");
            bot.exit();
        }
        other => state
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .events
            .on_event(&bot, &other),
    }
}
