//! Line-delimited JSON protocol between the executor's socket and its clients.

mod entity;
mod goal;
mod knobs;
mod reply;
mod request;

pub use entity::{entity_labels, parse_entity, ALL_KINDS};
pub use goal::{GoalName, GoalParams};
pub use knobs::{mode_label, WireGain, WireMode, WireResponse, WireScope, WireTrigger};
pub use reply::{
    err, ok, open, Ack, ChatLine, CompanionView, Events, GoalView, LogLine, LogPage, NearGroup,
    Observation, WakeEvent,
};
pub use request::Request;

/// Longest chat line the server accepts.
pub const CHAT_MAX: usize = 256;

/// Validates one outgoing chat line: single line, non-empty, at most `CHAT_MAX` chars.
pub fn chat_line(text: &str) -> Result<String, String> {
    let t = text.trim();
    if t.is_empty() {
        return Err("chat text is empty".into());
    }
    if t.contains(['\n', '\r']) {
        return Err("chat text must be one line".into());
    }
    if t.chars().count() > CHAT_MAX {
        return Err(format!("chat text longer than {CHAT_MAX} characters"));
    }
    Ok(t.to_string())
}
