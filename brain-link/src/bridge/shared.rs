use std::collections::VecDeque;
use std::sync::mpsc::SyncSender;

use reflex_core::events::Event;
use reflex_core::Snapshot;
use serde_json::Value;

use crate::wire::{ChatLine, Request, WakeEvent};

pub const CHAT_KEEP: usize = 50;
/// Unread WAKE events kept; the oldest are dropped first.
pub const WAKE_KEEP: usize = 64;

pub struct Job {
    pub req: Request,
    pub reply: SyncSender<Value>,
}

#[derive(Default)]
pub struct Shared {
    pub jobs: VecDeque<Job>,
    pub wakes: VecDeque<WakeEvent>,
    pub chat: VecDeque<ChatLine>,
}

impl Shared {
    pub fn hear(&mut self, snap: &Snapshot) {
        for c in &snap.chat {
            self.chat.push_back(ChatLine {
                t: snap.tick,
                from: c.sender.clone(),
                text: c.text.clone(),
            });
        }
        while self.chat.len() > CHAT_KEEP {
            self.chat.pop_front();
        }
    }

    pub fn push_wakes(&mut self, events: Vec<Event>) {
        self.wakes.extend(events.into_iter().map(|e| WakeEvent {
            t: e.tick,
            kind: e.kind,
            subject: e.subject,
            text: e.text,
        }));
        while self.wakes.len() > WAKE_KEEP {
            self.wakes.pop_front();
        }
    }
}
