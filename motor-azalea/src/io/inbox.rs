//! Line commands from stdin, read on a plain thread and drained once per tick.
//!
//! `goal <x> <y> <z> [priority] [spec...]`, `goal-end`, `mode <name>`, `say <text>`.

use std::{
    io::BufRead,
    sync::mpsc::{self, Receiver},
};

use reflex_core::{math::Vec3, Executor, Mode};

#[derive(Debug, PartialEq)]
pub enum BrainCmd {
    Goal {
        target: Vec3,
        priority: f32,
        spec: String,
    },
    GoalEnd,
    Mode(Mode),
    Say(String),
}

pub struct Inbox {
    rx: Receiver<BrainCmd>,
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
        Self { rx }
    }

    /// Applies every pending knob command; returns the chat lines to send.
    pub fn drain(&self, exec: &mut Executor) -> Vec<String> {
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
                BrainCmd::GoalEnd => {
                    exec.knobs.end_goal();
                }
                BrainCmd::Mode(m) => exec.knobs.set_mode(m),
                BrainCmd::Say(text) => say.push(text),
            }
        }
        say
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
        "say" if !rest.trim().is_empty() => Some(BrainCmd::Say(rest.trim().to_string())),
        _ => None,
    }
}
