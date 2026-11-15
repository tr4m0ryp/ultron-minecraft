use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::goal::GoalTick;

#[derive(Debug, Default)]
struct Inner {
    goal: Cell<GoalTick>,
    dump: RefCell<Option<String>>,
}

/// Shared between the goal runner (writes its state, asks for a dump) and the executor
/// (reads the state into its ring, renders the dump).
#[derive(Clone, Debug, Default)]
pub struct Probe(Rc<Inner>);

impl Probe {
    pub fn set(&self, g: GoalTick) {
        self.0.goal.set(g);
    }

    pub fn goal(&self) -> GoalTick {
        self.0.goal.get()
    }

    pub fn update(&self, f: impl FnOnce(&mut GoalTick)) {
        let mut g = self.0.goal.get();
        f(&mut g);
        self.0.goal.set(g);
    }

    /// Ask the executor to print the ring at the end of this tick (first reason wins).
    pub fn request_dump(&self, reason: String) {
        let mut d = self.0.dump.borrow_mut();
        if d.is_none() {
            *d = Some(reason);
        }
    }

    pub fn take_dump(&self) -> Option<String> {
        self.0.dump.borrow_mut().take()
    }
}
