use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

use azalea::prelude::*;
use reflex_core::{Behaviour, Executor};

use crate::{
    act::{self, HandState},
    io::{dump_trace, Inbox, Trace},
    sense::{self, EventBuffer, SenseState},
};

/// What lives on the handler thread only.
struct Local {
    exec: Executor,
    inbox: Inbox,
    trace: Trace,
    goals: reflex_core::goals::GoalHandle,
    ledger_watch: crate::ledger::Watch,
    link: Option<super::Link>,
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
        let Some(mut snap) = sense::snapshot(
            bot,
            &mut inner.sense,
            &mut inner.events,
            self.companion.as_deref(),
        ) else {
            return;
        };
        snap.me.shield_ticks = inner.hands.shield_ticks();
        let sensed = started.elapsed();
        let (out, say) = LOCAL.with_borrow_mut(|slot| {
            let l = slot.get_or_insert_with(|| {
                let (behaviours, goals) = crate::goal_catalog(bot);
                Local {
                    exec: executor(&bot.username(), behaviours),
                    inbox: Inbox::spawn_stdin(),
                    trace: Trace::default(),
                    goals,
                    ledger_watch: Default::default(),
                    link: super::Link::mount(),
                }
            });
            let mut say = l.inbox.drain(&mut l.exec);
            l.exec.knobs.kept = l.goals.ledger().keep();
            let out = l.exec.tick(&snap);
            l.ledger_watch.tick(bot, l.goals.ledger(), &snap, &out);
            crate::nav::set_max_drop(l.exec.knobs.gains.safe_drop());
            l.goals.settle(&mut l.exec.knobs);
            dump_trace(&mut l.exec);
            l.trace.tick(&snap, &l.exec, sensed);
            say.extend(super::link::publish(
                l.link.as_ref(),
                &mut l.exec,
                &l.goals,
                &snap,
            ));
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
fn executor(name: &str, catalog: Vec<Box<dyn Behaviour>>) -> Executor {
    let catalog = match std::env::var("MOTOR_ONLY") {
        Ok(only) => {
            let keep: Vec<&str> = only.split(',').map(str::trim).collect();
            catalog
                .into_iter()
                .filter(|b| keep.contains(&b.id()))
                .collect()
        }
        Err(_) => catalog,
    };
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
