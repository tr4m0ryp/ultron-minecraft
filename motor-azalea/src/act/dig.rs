//! Block digging done by the adapter: START, swing while the vanilla progress estimate
//! accumulates, then STOP. Azalea's mining plugin cannot be used against 26.3 (see
//! `protocol`). The client never predicts the break; the server's block update decides.

use azalea::{
    block::BlockTrait,
    core::{direction::Direction, position::BlockPos},
    entity::{
        inventory::Inventory, mining::get_mine_progress, ActiveEffects, Attributes, FluidOnEyes,
        Physics, Position,
    },
    local_player::WorldHolder,
    protocol::packets::game::{s_interact::InteractionHand, s_swing::ServerboundSwing},
    Client,
};

use super::protocol::{send, PlayerAction};

/// Ticks to wait for the server's block update after STOP before trying again.
const CONFIRM_TICKS: u32 = 10;

#[derive(Default)]
pub struct DigState {
    target: Option<BlockPos>,
    progress: f32,
    /// Ticks since STOP was sent; `None` while still digging.
    stopped: Option<u32>,
}

impl DigState {
    /// One tick. `want` is this tick's dig target, `None` when the main hand does something
    /// else (an unfinished dig is then aborted).
    pub fn tick(&mut self, bot: &Client, want: Option<reflex_core::BlockPos>) {
        let want = want.map(|p| BlockPos::new(p.x, p.y, p.z));
        if want != self.target {
            if let (Some(old), None) = (self.target, self.stopped) {
                send(bot, PlayerAction::AbortDestroy, old, Direction::Down);
            }
            *self = Self::default();
            let Some(p) = want else { return };
            if is_air(bot, p) {
                return;
            }
            self.target = Some(p);
            send(bot, PlayerAction::StartDestroy, p, face_toward(bot, p));
        }
        let Some(p) = self.target else { return };
        if is_air(bot, p) {
            *self = Self::default();
            return;
        }
        if let Some(t) = &mut self.stopped {
            *t += 1;
            if *t >= CONFIRM_TICKS {
                // Server disagreed; dig again from the start.
                *self = Self::default();
            }
            return;
        }
        bot.write_packet(ServerboundSwing {
            hand: InteractionHand::MainHand,
        });
        self.progress += rate(bot, p);
        if self.progress >= 1.0 {
            send(bot, PlayerAction::StopDestroy, p, face_toward(bot, p));
            self.stopped = Some(0);
        }
    }
}

fn is_air(bot: &Client, p: BlockPos) -> bool {
    let ecs = bot.ecs.read();
    let Some(world) = ecs.get::<WorldHolder>(bot.entity) else {
        return true;
    };
    let state = world.shared.read().get_block_state(p);
    state.is_none_or(|s| s.is_air())
}

/// Vanilla per-tick destroy progress for the held item, effects, fluid and ground state.
fn rate(bot: &Client, p: BlockPos) -> f32 {
    let ecs = bot.ecs.read();
    let e = bot.entity;
    let (Some(world), Some(inv), Some(fluid), Some(phys), Some(attr), Some(fx)) = (
        ecs.get::<WorldHolder>(e),
        ecs.get::<Inventory>(e),
        ecs.get::<FluidOnEyes>(e),
        ecs.get::<Physics>(e),
        ecs.get::<Attributes>(e),
        ecs.get::<ActiveEffects>(e),
    ) else {
        return 0.0;
    };
    let Some(state) = world.shared.read().get_block_state(p) else {
        return 0.0;
    };
    let block = Box::<dyn BlockTrait>::from(state);
    get_mine_progress(block.as_ref(), inv.held_item(), fluid, phys, attr, fx)
}

/// The block face pointing at the player, by the dominant axis of the offset.
fn face_toward(bot: &Client, p: BlockPos) -> Direction {
    let Ok(pos) = bot.component::<Position>() else {
        return Direction::Up;
    };
    let eye_y = pos.y + 1.62;
    let (dx, dy, dz) = (
        pos.x - (p.x as f64 + 0.5),
        eye_y - (p.y as f64 + 0.5),
        pos.z - (p.z as f64 + 0.5),
    );
    if dy.abs() >= dx.abs() && dy.abs() >= dz.abs() {
        if dy > 0.0 {
            Direction::Up
        } else {
            Direction::Down
        }
    } else if dx.abs() >= dz.abs() {
        if dx > 0.0 {
            Direction::East
        } else {
            Direction::West
        }
    } else if dz > 0.0 {
        Direction::South
    } else {
        Direction::North
    }
}
