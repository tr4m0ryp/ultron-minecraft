use std::collections::HashMap;

/// Weight below which a repeated event no longer reaches the brain.
pub const HABITUATED_BELOW: f32 = 0.2;
/// Recovery time constant: 2 minutes of game time.
pub const RECOVERY_TICKS: f64 = 2400.0;
/// Relative magnitude change that resets habituation (dishabituation).
const DISHABITUATE_RATIO: f32 = 0.5;

#[derive(Clone, Copy, Debug)]
struct Trace {
    weight: f32,
    last_tick: u64,
    last_magnitude: f32,
}

/// Per-key response weight that halves on each repeat and recovers exponentially.
#[derive(Clone, Debug, Default)]
pub struct Habituation {
    traces: HashMap<String, Trace>,
    /// Keys the brain asked to ignore, with expiry tick (seeds habituation).
    ignored: HashMap<String, u64>,
}

impl Habituation {
    /// Registers an occurrence and returns the weight it is reported with.
    pub fn observe(&mut self, key: &str, magnitude: f32, now: u64) -> f32 {
        if self.ignored.get(key).is_some_and(|until| now < *until)
            || self.ignored.iter().any(|(k, until)| now < *until && key.starts_with(k.as_str()))
        {
            return 0.0;
        }
        let t = self.traces.entry(key.to_string()).or_insert(Trace {
            weight: 1.0,
            last_tick: now,
            last_magnitude: magnitude,
        });
        let dt = now.saturating_sub(t.last_tick) as f64;
        t.weight = 1.0 - (1.0 - t.weight) * (-dt / RECOVERY_TICKS).exp() as f32;
        let base = t.last_magnitude.abs().max(1.0);
        if (magnitude - t.last_magnitude).abs() / base > DISHABITUATE_RATIO {
            t.weight = 1.0;
        }
        let reported = t.weight;
        t.weight *= 0.5;
        t.last_tick = now;
        t.last_magnitude = magnitude;
        reported
    }

    /// ignore(): suppress keys starting with `prefix` until `until`.
    pub fn ignore(&mut self, prefix: &str, until: u64) {
        self.ignored.insert(prefix.to_string(), until);
    }

    /// attend(): break habituation for keys starting with `prefix`.
    pub fn attend(&mut self, prefix: &str) {
        self.ignored.retain(|k, _| !k.starts_with(prefix));
        for (k, t) in self.traces.iter_mut() {
            if k.starts_with(prefix) {
                t.weight = 1.0;
            }
        }
    }
}
