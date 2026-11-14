//! Azalea block states as the route planner's cells, with the dig time for the tools in
//! the hotbar. Two readers share the classification: `PlanTerrain` caches whole chunks for
//! a search on the planner thread, `LiveTerrain` reads the world directly for the follower
//! on the tick thread.

use std::{cell::RefCell, collections::HashMap, sync::Arc};

use azalea::{
    auto_tool::best_tool_in_hotbar_for_block,
    block::{
        fluid_state::{FluidKind, FluidState},
        BlockState,
    },
    inventory::Menu,
    pathfinder::{
        positions::RelBlockPos,
        world::{is_block_state_passable, is_block_state_standable, CachedWorld},
    },
    physics::collision::BlockWithShape,
    registry::{builtin::BlockKind as Az, tags::blocks},
    world::World,
    Client,
};
use parking_lot::RwLock;
use reflex_core::{
    goals::travel::route::{natural, Cell, Terrain},
    math::BlockPos,
};

use crate::world::names::block_name;

fn falls(k: Az) -> bool {
    matches!(
        k,
        Az::Sand | Az::RedSand | Az::Gravel | Az::SuspiciousGravel | Az::SuspiciousSand
    ) || blocks::CONCRETE_POWDERS.contains(&k)
}

/// The cell for one block state; `menu` (the inventory) prices digging, `None` forbids it.
pub fn classify(state: BlockState, menu: Option<&Menu>) -> Cell {
    if is_block_state_passable(state) {
        return Cell::Open;
    }
    if state.is_collision_shape_empty() {
        return match FluidState::from(state).kind {
            FluidKind::Water => Cell::Water,
            FluidKind::Lava => Cell::Lava,
            FluidKind::Empty => Cell::Hazard,
        };
    }
    let k = Az::from(state);
    let dig = menu.filter(|_| natural(&block_name(state))).and_then(|m| {
        let pct = best_tool_in_hotbar_for_block(state, m).percentage_per_tick;
        (pct > 0.0).then(|| (1.0 / pct).ceil().min(f32::from(u16::MAX)) as u16)
    });
    Cell::Solid {
        floor: is_block_state_standable(state) && k != Az::MagmaBlock,
        dig,
        falls: falls(k),
    }
}

/// Cells cached per block state id; the dig time depends only on the state and the tools.
#[derive(Default)]
struct Cache(RefCell<HashMap<u32, Cell>>);

impl Cache {
    fn get(&self, state: BlockState, menu: Option<&Menu>) -> Cell {
        let id = state.id() as u32;
        if let Some(c) = self.0.borrow().get(&id) {
            return *c;
        }
        let c = classify(state, menu);
        self.0.borrow_mut().insert(id, c);
        c
    }
}

/// A search-time view: chunks copied on first use, so the world lock is held only briefly.
pub struct PlanTerrain {
    world: CachedWorld,
    lock: Arc<RwLock<World>>,
    /// Chunk columns seen, and whether the client has them (unloaded ones read as air).
    loaded: RefCell<HashMap<(i32, i32), bool>>,
    origin: azalea::BlockPos,
    menu: Option<Menu>,
    min_y: i32,
    max_y: i32,
    cache: Cache,
}

impl PlanTerrain {
    pub fn new(world: Arc<RwLock<World>>, origin: BlockPos, menu: Option<Menu>) -> Self {
        let (min_y, height) = {
            let w = world.read();
            (w.chunks.min_y(), w.chunks.height() as i32)
        };
        let origin = azalea::BlockPos::new(origin.x, origin.y, origin.z);
        Self {
            world: CachedWorld::new(world.clone(), origin),
            lock: world,
            loaded: RefCell::default(),
            origin,
            menu,
            min_y,
            max_y: min_y + height - 1,
            cache: Cache::default(),
        }
    }
}

impl Terrain for PlanTerrain {
    fn cell(&self, p: BlockPos) -> Cell {
        if p.y < self.min_y || p.y > self.max_y {
            return Cell::Unknown;
        }
        let (dx, dz) = (p.x - self.origin.x, p.z - self.origin.z);
        if dx.abs() > 30_000 || dz.abs() > 30_000 {
            return Cell::Unknown;
        }
        let rel = RelBlockPos::from_origin(self.origin, azalea::BlockPos::new(p.x, p.y, p.z));
        let state = self.world.get_block_state(rel);
        if state.is_air() && !self.loaded(p) {
            return Cell::Unknown;
        }
        self.cache.get(state, self.menu.as_ref())
    }
}

impl PlanTerrain {
    fn loaded(&self, p: BlockPos) -> bool {
        let key = (p.x.div_euclid(16), p.z.div_euclid(16));
        if let Some(&l) = self.loaded.borrow().get(&key) {
            return l;
        }
        let pos = azalea::core::position::ChunkPos::new(key.0, key.1);
        let l = self.lock.read().chunks.get(&pos).is_some();
        self.loaded.borrow_mut().insert(key, l);
        l
    }
}

/// A tick-time view straight from the client's world.
pub struct LiveTerrain<'a> {
    bot: &'a Client,
    cache: Cache,
}

impl<'a> LiveTerrain<'a> {
    pub fn new(bot: &'a Client) -> Self {
        Self {
            bot,
            cache: Cache::default(),
        }
    }
}

impl Terrain for LiveTerrain<'_> {
    fn cell(&self, p: BlockPos) -> Cell {
        let Ok(world) = self.bot.world() else {
            return Cell::Unknown;
        };
        let state = world
            .read()
            .get_block_state(azalea::BlockPos::new(p.x, p.y, p.z));
        match state {
            Some(s) => self.cache.get(s, None),
            None => Cell::Unknown,
        }
    }
}
