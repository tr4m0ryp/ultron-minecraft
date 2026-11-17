//! Line commands from stdin, read on a plain thread and drained once per tick.
//!
//! `goal <x> <y> <z> [priority] [spec...]`, `goal-end`, `mode <name>`, `gain <name> <value>`,
//! `say <text>`, and the block probes of `watch`.

use std::{
    io::BufRead,
    sync::mpsc::{self, Receiver},
};

use reflex_core::{math::Vec3, Executor, GainName, Mode};

use super::watch::{self, ProbeCmd};

#[derive(Debug, PartialEq)]
pub enum BrainCmd {
    Goal {
        target: Vec3,
        priority: f32,
        spec: String,
    },
    /// A goal-layer spec with no knob target, e.g. `task mine stone 3`.
    Task(String),
    GoalEnd,
    Mode(Mode),
    Gain(GainName, f32),
    Say(String),
    /// A staged measurement's block probe; answered by the tick loop, not the executor.
    Probe(ProbeCmd),
}

pub struct Inbox {
    rx: Receiver<BrainCmd>,
    probes: Vec<ProbeCmd>,
}

impl Inbox {
    /// Starts the stdin reader thread. Unparseable lines are reported on stderr and skipped.
    pub fn spawn_stdin() -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for line in std::io::stdin().lock().lines().map_while(Result::ok) {
                match parse(&line) {
                    Some(cmd) => {
                        if tx.send(cmd).is_err() {
                            break;
                        }
                    }
                    None if line.trim().is_empty() => {}
                    None => eprintln!("brain: cannot parse {line:?}"),
                }
            }
        });
        Self {
            rx,
            probes: Vec::new(),
        }
    }

    /// Applies every pending knob command; returns the chat lines to send.
    pub fn drain(&mut self, exec: &mut Executor) -> Vec<String> {
        let mut say = Vec::new();
        while let Ok(cmd) = self.rx.try_recv() {
            match cmd {
                BrainCmd::Goal {
                    target,
                    priority,
                    spec,
                } => {
                    exec.knobs.set_goal(spec, priority, Some(target), false);
                }
                BrainCmd::Task(spec) => {
                    let solo = !matches!(
                        spec.split_whitespace().next(),
                        Some("follow" | "come_here" | "come" | "guard" | "give" | "eat_now")
                    );
                    exec.knobs.set_goal(spec, 0.6, None, solo);
                }
                BrainCmd::GoalEnd => {
                    exec.knobs.end_goal();
                }
                BrainCmd::Mode(m) => exec.knobs.set_mode(m),
                BrainCmd::Gain(name, v) => exec.knobs.set_gain(name, v),
                BrainCmd::Say(text) => say.push(text),
                BrainCmd::Probe(p) => self.probes.push(p),
            }
        }
        say
    }

    /// Block probes that arrived since the last call.
    pub fn take_probes(&mut self) -> Vec<ProbeCmd> {
        std::mem::take(&mut self.probes)
    }
}

pub fn parse(line: &str) -> Option<BrainCmd> {
    let line = line.trim();
    let (head, rest) = line.split_once(' ').unwrap_or((line, ""));
    match head {
        "goal" => {
            let mut it = rest.split_whitespace();
            let mut num = || it.next()?.parse::<f64>().ok();
            let target = Vec3::new(num()?, num()?, num()?);
            let priority = num().unwrap_or(0.5) as f32;
            let spec: Vec<&str> = it.collect();
            let spec = if spec.is_empty() {
                "walk".to_string()
            } else {
                spec.join(" ")
            };
            Some(BrainCmd::Goal {
                target,
                priority,
                spec,
            })
        }
        "goal-end" => Some(BrainCmd::GoalEnd),
        "task" if !rest.trim().is_empty() => Some(BrainCmd::Task(rest.trim().to_string())),
        "mode" => Some(BrainCmd::Mode(match rest.trim() {
            "explore" => Mode::Explore,
            "mine" => Mode::Mine,
            "build" => Mode::Build,
            "fight" => Mode::Fight,
            "retreat" => Mode::Retreat,
            "follow" => Mode::Follow,
            "idle" => Mode::IdleSafe,
            _ => return None,
        })),
        "gain" => {
            let mut it = rest.split_whitespace();
            let name = match it.next()? {
                "aggression" => GainName::Aggression,
                "caution" => GainName::Caution,
                "hp_floor" => GainName::HpFloor,
                "hunger_floor" => GainName::HungerFloor,
                "leash" => GainName::Leash,
                "vigilance" => GainName::Vigilance,
                "air_floor" => GainName::AirFloor,
                "mlg_lead" => GainName::MlgLead,
                _ => return None,
            };
            Some(BrainCmd::Gain(name, it.next()?.parse().ok()?))
        }
        "say" if !rest.trim().is_empty() => Some(BrainCmd::Say(rest.trim().to_string())),
        _ => watch::parse(line).map(BrainCmd::Probe),
    }
}
