//! A tiny world simulator: applies the executor's motor output to a flat arena snapshot and
//! to the fake world, so goals run end to end through the real selector and reflexes.
#![allow(dead_code)]

mod physics;

use reflex_core::events::Event;
use reflex_core::goals::fakes::{FakeNav, FakeWorld};
use reflex_core::goals::{catalog, GoalHandle};
use reflex_core::snapshot::{
    BlockKind, Companion, Entity, EntityKind, ItemKind, ItemStack, Snapshot,
};
use reflex_core::{BlockPos, Executor, MotorOutput, Vec3};

pub const FLOOR_Y: i32 = 63;
pub const COMPANION_ID: u32 = 99;
const R: i32 = 30;

pub struct Sim {
    pub exec: Executor,
    pub snap: Snapshot,
    pub nav: FakeNav,
    pub world: FakeWorld,
    pub handle: GoalHandle,
    /// Where a `PlaceOn` puts the held block.
    pub place_at: Option<BlockPos>,
    pub dig_ticks: u32,
    pub hits_to_kill: u32,
    pub attacked: Vec<u32>,
    pub last: MotorOutput,
    dig: Option<(BlockPos, u32)>,
    eat_ticks: u32,
    hits: std::collections::HashMap<u32, u32>,
}

impl Sim {
    pub fn new() -> Self {
        let mut snap = Snapshot {
            tick: 1000,
            world_min_y: -64,
            ..Default::default()
        };
        snap.blocks.fill(
            BlockPos::new(-R, FLOOR_Y - 4, -R),
            BlockPos::new(R, FLOOR_Y, R),
            BlockKind::Solid,
        );
        snap.blocks.fill(
            BlockPos::new(-R, FLOOR_Y + 1, -R),
            BlockPos::new(R, FLOOR_Y + 4, R),
            BlockKind::Air,
        );
        snap.me.pos = Vec3::new(0.5, 64.0, 0.5);
        let (nav, world, handle) = (
            FakeNav::default(),
            FakeWorld::default(),
            GoalHandle::default(),
        );
        world.state().free_slots = 20;
        world.state().ground_y = Some(FLOOR_Y);
        let behaviours = catalog(
            Box::new(nav.clone()),
            Box::new(world.clone()),
            handle.clone(),
        );
        Self {
            exec: Executor::with_behaviours("Ultron", behaviours),
            snap,
            nav,
            world,
            handle,
            place_at: None,
            dig_ticks: 10,
            hits_to_kill: 3,
            attacked: Vec::new(),
            last: MotorOutput::default(),
            dig: None,
            eat_ticks: 0,
            hits: Default::default(),
        }
    }

    pub fn goal(&mut self, spec: &str, solo: bool) -> u64 {
        self.exec.knobs.set_goal(spec, 0.6, None, solo)
    }

    pub fn companion_at(&mut self, x: f64, z: f64) {
        self.snap.companion = Some(Companion {
            name: "Steve".into(),
            entity_id: Some(COMPANION_ID),
            pos: Vec3::new(x, 64.0, z),
            vel: Vec3::ZERO,
            health: Some(20.0),
        });
    }

    pub fn mob(&mut self, id: u32, kind: EntityKind, x: f64, z: f64) {
        self.snap
            .entities
            .push(Entity::new(id, kind, Vec3::new(x, 64.0, z)));
    }

    pub fn block(&mut self, p: BlockPos, name: &str) {
        self.world.state().blocks.insert(p, name.to_string());
        self.snap.blocks.set(p, BlockKind::Solid);
    }

    pub fn hotbar(&mut self, slot: usize, kind: ItemKind, count: u8) {
        self.snap.me.hotbar[slot] = Some(ItemStack::new(kind, count));
    }

    pub fn item(&mut self, name: &str, n: u32, slot: Option<u8>) {
        let mut w = self.world.state();
        w.add_item(name, n);
        if let Some(s) = slot {
            w.slots.insert(name.to_string(), s);
        }
    }

    pub fn step(&mut self) -> MotorOutput {
        self.snap.tick += 1;
        let out = self.exec.tick(&self.snap);
        self.snap.damage.clear();
        self.apply(&out);
        self.handle.settle(&mut self.exec.knobs);
        self.last = out.clone();
        out
    }

    /// Steps until `done` holds or `max` ticks pass; returns whether it held.
    pub fn run_until(&mut self, max: u32, mut done: impl FnMut(&Sim) -> bool) -> bool {
        for _ in 0..max {
            self.step();
            if done(self) {
                return true;
            }
        }
        false
    }

    pub fn events(&self, kind: &str) -> Vec<Event> {
        self.exec
            .upward
            .log()
            .filter(|e| e.kind == kind)
            .cloned()
            .collect()
    }

    pub fn has_event(&self, kind: &str) -> bool {
        !self.events(kind).is_empty()
    }

    pub fn goal_text(&self, kind: &str) -> String {
        self.events(kind)
            .last()
            .map(|e| e.text.clone())
            .unwrap_or_default()
    }
}
