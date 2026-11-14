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
            let l = slot.get_or_insert_with(|| Local {
                exec: Executor::new(&bot.username()),
                inbox: Inbox::spawn_stdin(),
                trace: Trace::default(),
            });
            let say = l.inbox.drain(&mut l.exec);
            let out = l.exec.tick(&snap);
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
