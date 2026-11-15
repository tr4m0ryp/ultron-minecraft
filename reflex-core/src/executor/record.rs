//! Flight-recorder write path: one `TickRec` per tick from the selector's allocation.

use std::collections::HashMap;

use super::Executor;
use crate::channels::Channel;
use crate::snapshot::Snapshot;
use crate::trace::{Decision, Probe, TickRec, VETO_SLOTS};

impl Executor {
    /// Writes this tick's decisions into the ring and renders it if the goal asked.
    pub(super) fn record(
        &mut self,
        snap: &Snapshot,
        owners: &HashMap<Channel, usize>,
        owners_idx: &[Option<usize>],
        vetoes: [&'static str; VETO_SLOTS],
    ) {
        let owner = |c: Channel| {
            owners.get(&c).map_or("", |&i| match owners_idx[i] {
                Some(k) => self.behaviours[k].id(),
                None => "ARM",
            })
        };
        let d = Decision {
            owners: [
                owner(Channel::Legs),
                owner(Channel::Gaze),
                owner(Channel::Main),
                owner(Channel::Off),
                owner(Channel::Inv),
            ],
            legs_note: self.legs_note.unwrap_or(""),
            vetoes,
            goal: self.probe.as_ref().map(Probe::goal).unwrap_or_default(),
        };
        let p = snap.me.pos;
        self.ring.push(TickRec {
            tick: snap.tick,
            pos: [p.x as f32, p.y as f32, p.z as f32],
            d,
        });
        if let Some(reason) = self.probe.as_ref().and_then(Probe::take_dump) {
            self.dump
                .extend(self.ring.render(&format!("t={} {reason}", snap.tick)));
        }
    }
}
