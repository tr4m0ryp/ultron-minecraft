use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::state::State;
use crate::wire::Request;

/// Version of the line format; raise it when a field changes meaning.
pub const FORMAT: u32 = 1;

/// What the body answered: the acknowledgement line, or why it refused.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Outcome {
    pub ok: bool,
    pub text: String,
    /// The id the decision got (goal, inhibition or armed response), when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
}

impl Outcome {
    /// From the reply envelope `{"ok":true,"data":{"id":..,"done":".."}}` or its error form.
    pub fn of(reply: &Value) -> Self {
        let ok = reply["ok"].as_bool().unwrap_or(false);
        let text = match ok {
            true => reply["data"]["done"].as_str(),
            false => reply["error"].as_str(),
        };
        Self {
            ok,
            text: text.unwrap_or_default().to_string(),
            id: reply["data"]["id"].as_u64(),
        }
    }
}

/// One line of the log.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub v: u32,
    /// The game tick the decision was applied in; `state` is that tick's snapshot.
    pub tick: u64,
    /// Wall clock, milliseconds since the Unix epoch, to line the log up with transcripts.
    pub wall_ms: u64,
    pub state: State,
    /// The call as it arrived, with every parameter (`{"op":"set_goal","goal":..}`).
    pub call: Request,
    pub result: Outcome,
}
