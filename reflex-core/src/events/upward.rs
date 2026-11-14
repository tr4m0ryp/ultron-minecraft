use std::collections::{HashMap, VecDeque};

use super::digest::{self, Digest};
use super::event::{Event, EventClass};
use super::habituation::{Habituation, HABITUATED_BELOW};

/// WAKE at most once per 5 s (100 ticks) except critical events.
pub const WAKE_MIN_GAP_TICKS: u64 = 100;
const LOG_CAP: usize = 4096;

#[derive(Clone, Debug, Default)]
pub struct Upward {
    pub habituation: Habituation,
    /// What the brain was last told per key: an event repeating it is no prediction error.
    told: HashMap<String, String>,
    pending_digest: Vec<(Event, f32)>,
    digest_since: u64,
    wake_queue: Vec<Event>,
    last_wake: Option<u64>,
    log: VecDeque<Event>,
}

impl Upward {
    pub fn push(&mut self, ev: Event) {
        self.log.push_back(ev.clone());
        if self.log.len() > LOG_CAP {
            self.log.pop_front();
        }
        if ev.class == EventClass::Log || ev.self_caused {
            return;
        }
        let key = ev.key();
        if !ev.text.is_empty() && self.told.get(&key) == Some(&ev.text) && !ev.critical {
            return;
        }
        match ev.class {
            EventClass::Wake => {
                self.told.insert(key, ev.text.clone());
                self.wake_queue.push(ev);
            }
            EventClass::Digest => {
                let w = self.habituation.observe(&key, ev.magnitude, ev.tick);
                if w < HABITUATED_BELOW {
                    return;
                }
                self.told.insert(key, ev.text.clone());
                if self.pending_digest.is_empty() {
                    self.digest_since = ev.tick;
                }
                self.pending_digest.push((ev, w));
            }
            EventClass::Log => {}
        }
    }

    /// WAKE events due now. Non-critical wakes inside the rate-limit window stay queued.
    pub fn take_wake(&mut self, now: u64) -> Vec<Event> {
        if self.wake_queue.is_empty() {
            return Vec::new();
        }
        let gap_ok = self.last_wake.is_none_or(|t| now >= t + WAKE_MIN_GAP_TICKS);
        if !gap_ok && !self.wake_queue.iter().any(|e| e.critical) {
            return Vec::new();
        }
        self.last_wake = Some(now);
        std::mem::take(&mut self.wake_queue)
    }

    pub fn has_wake(&self) -> bool {
        !self.wake_queue.is_empty()
    }

    /// Drains pending DIGEST events into one token-capped digest.
    pub fn take_digest(&mut self, cap: usize) -> Digest {
        let d = digest::build(&self.pending_digest, self.digest_since, cap);
        self.pending_digest.clear();
        d
    }

    pub fn log(&self) -> impl DoubleEndedIterator<Item = &Event> {
        self.log.iter()
    }
}
