//! Azalea client state -> `reflex_core::snapshot::Snapshot`.

pub(crate) mod block_kind;
mod blocks;
mod body;
mod cadence;
mod entities;
mod entity_kind;
mod events;
mod items;

use std::collections::HashMap;

use azalea::{local_player::WorldHolder, Client};
use reflex_core::{
    math::Vec3,
    snapshot::{ChatEvent, DamageEvent, LocalBlocks, Snapshot},
};

pub use body::menu_index;
pub use events::EventBuffer;

/// Per-bot memory the snapshot builder carries between ticks.
#[derive(Default)]
pub struct SenseState {
    tick: u64,
    prev_entity_pos: HashMap<i32, Vec3>,
    prev_hp: Option<f32>,
    cadence: cadence::Cadence,
}

/// Build one snapshot, draining `events`. `None` while the bot is not yet in a loaded world.
pub fn snapshot(
    bot: &Client,
    st: &mut SenseState,
    events: &mut EventBuffer,
    companion: Option<&str>,
) -> Option<Snapshot> {
    let mut ecs = bot.ecs.write();
    let me_state = body::read(&ecs, bot.entity)?;
    let world = ecs.get::<WorldHolder>(bot.entity)?.shared.clone();
    let mut view = entities::read(
        &mut ecs,
        bot.entity,
        me_state.pos,
        &mut st.prev_entity_pos,
        companion,
    )?;
    drop(ecs);

    let mut blocks = LocalBlocks::default();
    let world = world.read();
    blocks::read(&world, me_state.pos.block(), &mut blocks);
    let world_min_y = blocks::world_min_y(&world);
    drop(world);

    let hp = me_state.hp_total();
    let lost = st.prev_hp.map_or(0.0, |p| (p - hp).max(0.0));
    st.prev_hp = Some(hp);

    let find = |id: u32| view.entities.iter().find(|e| e.id == id).map(|e| e.pos);
    let mut damage: Vec<DamageEvent> = std::mem::take(&mut events.damage)
        .into_iter()
        .map(|d| DamageEvent {
            amount: 0.0,
            source_pos: d.source_pos.or_else(|| d.attacker.and_then(find)),
            attacker: d.attacker,
        })
        .collect();
    // The damage packet carries no amount; the health lost this tick is split across the
    // packets, or reported as one source-less event (fall, fire, drowning, starvation).
    if damage.is_empty() && lost > 0.0 {
        damage.push(DamageEvent {
            amount: lost,
            source_pos: None,
            attacker: None,
        });
    } else if !damage.is_empty() {
        let share = lost / damage.len() as f32;
        damage.iter_mut().for_each(|d| d.amount = share);
    }

    let chat = std::mem::take(&mut events.chat)
        .into_iter()
        .map(|c| {
            let sender_pos = view
                .entities
                .iter()
                .find(|e| e.name.as_deref() == Some(c.sender.as_str()))
                .map(|e| e.pos);
            ChatEvent {
                sender: c.sender,
                text: c.text,
                sender_pos,
            }
        })
        .collect();

    st.tick += 1;
    st.cadence.update(st.tick, &mut view.entities);
    Some(Snapshot {
        tick: st.tick,
        me: me_state,
        blocks,
        entities: view.entities,
        companion: view.companion,
        chat,
        damage,
        sounds: std::mem::take(&mut events.sounds),
        world_min_y,
        is_night: events.is_night(),
        day_time: events.day_time().map(|t| (t % 24_000) as u32),
        companion_name: companion.map(str::to_string),
    })
}
