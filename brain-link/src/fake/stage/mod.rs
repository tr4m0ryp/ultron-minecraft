//! Staging for brain sessions against the fake world: a control file the test driver
//! appends commands to (chat from the companion, goal failure rules, night, mobs), a goal
//! simulator that finishes or fails goals the way the goal layer reports them, and a
//! wall-clock log on stderr (`@<unix ms> ...`) from which reply latency is measured.
//!
//! Control lines: `chat <text>`, `fail <goal> <kind|*> <times> <reason>`, `night`, `day`,
//! `spawn <entity> <dx> <dz>`.

mod goals;
mod inbox;

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use reflex_core::events::{Event, EventClass};
use reflex_core::math::Vec3;
use reflex_core::snapshot::{ChatEvent, Entity, EntityKind};

use super::world::{FakeWorld, COMPANION};
use goals::{FailRule, GoalSim};
use inbox::Inbox;

pub struct Stage {
    inbox: Inbox,
    goals: GoalSim,
    next_entity: u32,
}

/// Unix time in milliseconds, the clock every stage log line carries.
pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// One stage log line on stderr.
pub fn note(what: &str) {
    eprintln!("brain-link-fake: @{} {what}", now_ms());
}

impl Stage {
    pub fn new(control: PathBuf) -> Self {
        Self {
            inbox: Inbox::new(control),
            goals: GoalSim::default(),
            next_entity: 100,
        }
    }

    /// Runs after `world.step()` and before the bridge pump.
    pub fn tick(&mut self, world: &mut FakeWorld) {
        for line in self.inbox.read_new() {
            self.command(world, line.trim());
        }
        self.goals.tick(world);
    }

    fn command(&mut self, world: &mut FakeWorld, line: &str) {
        let (verb, rest) = line.split_once(' ').unwrap_or((line, ""));
        match verb {
            "chat" if !rest.is_empty() => {
                let at = world.snap.companion.as_ref().map(|c| c.pos);
                world.snap.chat.push(ChatEvent {
                    sender: COMPANION.into(),
                    text: rest.into(),
                    sender_pos: at,
                });
                let text = format!("<{COMPANION}> {rest}");
                let e = Event::new(world.snap.tick, EventClass::Wake, "chat", COMPANION);
                world.exec.upward.push(e.text(text).critical()); // as orient::speaker does
                note(&format!("chat in: {rest}"));
            }
            "fail" => match FailRule::parse(rest) {
                Some(rule) => {
                    note(&format!("rule: fail {rest}"));
                    self.goals.rules.push(rule);
                }
                None => note(&format!("bad control line: {line}")),
            },
            "night" | "day" => {
                world.snap.is_night = verb == "night";
                note(verb);
            }
            "spawn" => self.spawn(world, rest),
            "" => {}
            _ => note(&format!("bad control line: {line}")),
        }
    }

    fn spawn(&mut self, world: &mut FakeWorld, rest: &str) {
        let mut it = rest.split_whitespace();
        let kind = match it.next() {
            Some("creeper") => EntityKind::Creeper,
            Some("zombie") => EntityKind::Zombie,
            Some("skeleton") => EntityKind::Skeleton,
            Some("spider") => EntityKind::Spider,
            _ => return note(&format!("bad spawn: {rest}")),
        };
        let dx: f64 = it.next().and_then(|v| v.parse().ok()).unwrap_or(10.0);
        let dz: f64 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
        let p = world.snap.me.pos + Vec3::new(dx, 0.0, dz);
        self.next_entity += 1;
        world
            .snap
            .entities
            .push(Entity::new(self.next_entity, kind, p));
        let name = rest.split_whitespace().next().unwrap_or("mob");
        let text = format!("{name} {:.0} blocks away", (dx * dx + dz * dz).sqrt());
        world.push(EventClass::Wake, "first_sighting", name, &text);
        note(&format!("spawn {rest}"));
    }
}
