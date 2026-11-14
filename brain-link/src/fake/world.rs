use reflex_core::behaviours::GOAL_ID;
use reflex_core::events::{Event, EventClass};
use reflex_core::math::Vec3;
use reflex_core::snapshot::{
    ChatEvent, Companion, Entity, EntityKind, ItemKind, ItemStack, Snapshot,
};

use super::FakeExecutor;

pub const BOT_NAME: &str = "Ultron";
pub const COMPANION: &str = "Steve";
const STEP: f64 = 0.25;
/// Scripted moments, in ticks, used by `FakeWorld::scripted`.
const T_DIGEST: u64 = 20;
const T_CHAT: u64 = 60;
const T_ZOMBIE: u64 = 400;

pub struct FakeWorld {
    pub exec: FakeExecutor,
    pub snap: Snapshot,
    scripted: bool,
    echo: Vec<String>,
    reached: Option<u64>,
}

impl FakeWorld {
    /// A quiet world: nothing happens unless the goal moves the bot.
    pub fn new() -> Self {
        let mut snap = Snapshot {
            tick: 0,
            world_min_y: -64,
            ..Default::default()
        };
        snap.me.pos = Vec3::new(0.5, 64.0, 0.5);
        snap.me.food = 18;
        snap.me.hotbar[0] = Some(ItemStack::new(ItemKind::Sword, 1));
        snap.companion = Some(Companion {
            name: COMPANION.into(),
            entity_id: Some(2),
            pos: Vec3::new(3.5, 64.0, 0.5),
            vel: Vec3::ZERO,
            health: Some(20.0),
        });
        snap.entities = vec![
            Entity::new(2, EntityKind::Player, Vec3::new(3.5, 64.0, 0.5)),
            Entity::new(3, EntityKind::Passive, Vec3::new(9.5, 64.0, -4.5)),
            Entity::new(4, EntityKind::Passive, Vec3::new(11.5, 64.0, -5.5)),
        ];
        Self {
            exec: FakeExecutor::default(),
            snap,
            scripted: false,
            echo: Vec::new(),
            reached: None,
        }
    }

    /// The demo world: a digest at 1 s, the companion walks off and calls at 3 s, a zombie
    /// shows up at 20 s.
    pub fn scripted() -> Self {
        Self {
            scripted: true,
            ..Self::new()
        }
    }

    /// Lines the bridge asked to say; they come back as chat on the next tick.
    pub fn hear(&mut self, said: Vec<String>) {
        self.echo.extend(said);
    }

    pub fn push(&mut self, class: EventClass, kind: &str, subject: &str, text: &str) {
        let e = Event::new(self.snap.tick, class, kind, subject).text(text);
        self.exec.upward.push(e);
    }

    /// Advances one tick and returns the new snapshot.
    pub fn step(&mut self) -> &Snapshot {
        self.snap.tick += 1;
        self.snap.chat = self
            .echo
            .drain(..)
            .map(|text| ChatEvent {
                sender: BOT_NAME.into(),
                text,
                sender_pos: None,
            })
            .collect();
        if self.scripted {
            self.script();
        }
        self.walk();
        &self.snap
    }

    fn script(&mut self) {
        match self.snap.tick {
            T_DIGEST => {
                self.push(EventClass::Digest, "edge_guard", "", "R5 edge_guard");
                self.push(EventClass::Digest, "eat", "", "H1 eat");
            }
            T_CHAT => {
                let to = Vec3::new(12.5, 64.0, 4.5);
                if let Some(c) = self.snap.companion.as_mut() {
                    c.pos = to;
                }
                self.snap.entities[0].pos = to;
                let line = "Ultron, come here";
                self.snap.chat.push(ChatEvent {
                    sender: COMPANION.into(),
                    text: line.into(),
                    sender_pos: Some(to),
                });
                let text = format!("<{COMPANION}> {line}");
                self.push(EventClass::Wake, "chat", COMPANION, &text);
            }
            T_ZOMBIE => {
                let p = self.snap.me.pos + Vec3::new(14.0, 0.0, 0.0);
                self.snap
                    .entities
                    .push(Entity::new(5, EntityKind::Zombie, p));
                self.push(
                    EventClass::Wake,
                    "first_sighting",
                    "zombie",
                    "first zombie this session",
                );
            }
            _ => {}
        }
    }

    fn walk(&mut self) {
        let me = self.snap.me.pos;
        let goal = self.exec.knobs.goal.clone();
        let leash = self.exec.knobs.gains.leash;
        let (dest, owner, stop_at) = match &goal {
            Some(g) if g.target.is_some() => (g.target, GOAL_ID, 1.0),
            Some(_) => (self.snap.companion.as_ref().map(|c| c.pos), "H8", leash),
            None => (None, "", 0.0),
        };
        let Some(dest) = dest else {
            self.exec.legs_owner = None;
            return;
        };
        let d = (dest - me).horizontal();
        if d.len() <= stop_at {
            self.exec.legs_owner = None;
            if let Some(g) = goal.filter(|g| g.target.is_some() && self.reached != Some(g.id)) {
                self.reached = Some(g.id);
                self.push(EventClass::Wake, "goal_reached", "", &g.spec);
            }
            return;
        }
        self.snap.me.pos = me + d.normalized() * STEP.min(d.len());
        self.exec.legs_owner = Some(owner.into());
    }
}

impl Default for FakeWorld {
    fn default() -> Self {
        Self::new()
    }
}
