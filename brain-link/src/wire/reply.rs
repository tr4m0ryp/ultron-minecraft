//! Reply payloads. The envelope is `{"ok":true,"data":...}` or `{"ok":false,"error":"..."}`.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub fn ok(data: impl Serialize) -> Value {
    json!({"ok": true, "data": data})
}

pub fn err(msg: impl Into<String>) -> Value {
    json!({"ok": false, "error": msg.into()})
}

/// Splits an envelope into its data or its error message.
pub fn open(reply: Value) -> Result<Value, String> {
    match reply.get("ok").and_then(Value::as_bool) {
        Some(true) => Ok(reply.get("data").cloned().unwrap_or(Value::Null)),
        Some(false) => Err(reply
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("unknown error")
            .to_string()),
        None => Err(format!("malformed reply: {reply}")),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub tick: u64,
    pub pos: [f64; 3],
    pub hp: f32,
    pub food: u8,
    pub held: Option<String>,
    pub night: bool,
    pub mode: String,
    /// Nearby entities grouped by kind, nearest group first.
    pub near: Vec<NearGroup>,
    pub companion: Option<CompanionView>,
    pub goal: Option<GoalView>,
    /// Outcome of the most recent goal that ended, e.g. `#3 mine done 3/3: mined 3 stone`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_goal: Option<String>,
    pub digest: String,
    pub wakes_pending: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NearGroup {
    pub kind: String,
    pub n: usize,
    pub dist: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompanionView {
    pub name: String,
    pub dist: f64,
    pub hp: Option<f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GoalView {
    pub id: u64,
    pub spec: String,
    pub priority: f32,
    /// running, blocked, preempted (paused_by:<behaviour id> when known), arrived or idle.
    pub status: String,
    /// Units done out of the total for counted goals, e.g. `2/3`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<String>,
    /// Latest progress note or blocked reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub queued: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WakeEvent {
    pub t: u64,
    pub kind: String,
    pub subject: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Events {
    pub events: Vec<WakeEvent>,
    pub timed_out: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatLine {
    pub t: u64,
    pub from: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogLine {
    pub t: u64,
    /// W wake, D digest, L log.
    pub class: char,
    pub kind: String,
    pub subject: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogPage {
    pub page: usize,
    pub total: usize,
    pub lines: Vec<LogLine>,
}

/// Acknowledgement of a knob change: the id it got and a one-line summary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ack {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    pub done: String,
}
