use serde::{Deserialize, Serialize};

use super::goal::{GoalName, GoalParams};
use super::knobs::{WireGain, WireMode, WireResponse, WireScope, WireTrigger};

/// One request line on the socket. `op` selects the variant.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Observe,
    WaitEvent {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timeout_s: Option<f64>,
    },
    SetGoal {
        goal: GoalName,
        #[serde(default)]
        params: GoalParams,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        priority: Option<f32>,
    },
    SetMode {
        mode: WireMode,
    },
    SetGain {
        gain: WireGain,
        value: f32,
    },
    Inhibit {
        reflex: String,
        #[serde(default)]
        scope: WireScope,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        strength: Option<f32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ttl_s: Option<f64>,
    },
    Arm {
        trigger: WireTrigger,
        response: WireResponse,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ttl_s: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        once: Option<bool>,
    },
    Say {
        text: String,
    },
    /// Mark a box the body must never break (min and max corners), or with `clear`, lift
    /// protection from every box containing that point.
    Protect {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<[f64; 3]>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<[f64; 3]>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        clear: Option<[f64; 3]>,
    },
    RecentChat {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<usize>,
    },
    Log {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        page: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        page_size: Option<usize>,
    },
}

impl Request {
    /// Requests answered by the tick loop rather than by the socket thread.
    pub fn needs_tick(&self) -> bool {
        !matches!(self, Request::WaitEvent { .. } | Request::RecentChat { .. })
    }
}
