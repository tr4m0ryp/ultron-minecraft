use std::sync::{Arc, Mutex};

use azalea::prelude::*;
use reflex_core::Executor;

use crate::{
    act::{self, HandState},
    sense::{self, EventBuffer, SenseState},
};

#[derive(Default)]
struct Inner {
    executor: Option<Executor>,
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
        Self { inner: Arc::default(), companion: companion.map(Into::into) }
    }

    fn tick(&self, bot: &Client) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let inner = &mut *guard;
        let Some(snap) =
            sense::snapshot(bot, &mut inner.sense, &mut inner.events, self.companion.as_deref())
        else {
            return;
        };
        let exec = inner.executor.get_or_insert_with(|| Executor::new(&bot.username()));
        let out = exec.tick(&snap);
        act::apply(bot, &out, &snap.me, &mut inner.hands);
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
