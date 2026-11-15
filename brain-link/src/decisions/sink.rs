//! Where the log goes: one file, appended to, a line written out whole per decision. The
//! write is buffered by the operating system and not synced: a decision arrives a few times
//! a minute, and a crash may lose the last lines but never tears one already flushed.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use reflex_core::Snapshot;
use serde_json::Value;

use super::record::{Outcome, Record, FORMAT};
use super::state::State;
use crate::host::Host;
use crate::wire::Request;

/// Names the log file. `off` (or `none`, `0`) writes none. Unset: the play state directory,
/// `logs/play/decisions.jsonl` under the working directory. Staged runs set their own file.
pub const DECISIONS_ENV: &str = "ULTRON_DECISIONS";
const DEFAULT_PATH: &str = "logs/play/decisions.jsonl";

pub fn default_path() -> Option<PathBuf> {
    match std::env::var(DECISIONS_ENV) {
        Ok(v) if matches!(v.trim(), "off" | "none" | "0") => None,
        Ok(v) if !v.trim().is_empty() => Some(PathBuf::from(v.trim())),
        _ => Some(PathBuf::from(DEFAULT_PATH)),
    }
}

/// What the log has cost the tick thread so far.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Cost {
    pub records: u64,
    pub total: Duration,
    pub max: Duration,
    pub last: Duration,
}

#[derive(Default)]
pub struct Sink {
    file: Option<File>,
    cost: Cost,
}

impl Sink {
    /// A sink appending to `path` (its directory is created). Errors leave the log off.
    pub fn open(path: &Path) -> std::io::Result<Self> {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            fs::create_dir_all(dir)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            file: Some(file),
            cost: Cost::default(),
        })
    }

    pub fn is_on(&self) -> bool {
        self.file.is_some()
    }

    pub fn cost(&self) -> Cost {
        self.cost
    }

    /// The state to record for a decision about to be applied; `None` when the log is off.
    pub fn before<H: Host>(&self, host: &mut H, snap: &Snapshot) -> Option<(State, Instant)> {
        let t0 = Instant::now();
        self.is_on().then(|| (State::take(host, snap), t0))
    }

    /// Appends the record of `call`, applied at `tick` with `reply`.
    pub fn after(&mut self, taken: (State, Instant), tick: u64, call: Request, reply: &Value) {
        let (state, t0) = taken;
        let record = Record {
            v: FORMAT,
            tick,
            wall_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_millis() as u64),
            state,
            call,
            result: Outcome::of(reply),
        };
        if let (Some(f), Ok(mut line)) = (self.file.as_mut(), serde_json::to_string(&record)) {
            line.push('\n');
            if let Err(e) = f.write_all(line.as_bytes()) {
                eprintln!("decision log not written: {e}");
                self.file = None;
            }
        }
        let spent = t0.elapsed();
        self.cost.records += 1;
        self.cost.total += spent;
        self.cost.max = self.cost.max.max(spent);
        self.cost.last = spent;
    }
}
