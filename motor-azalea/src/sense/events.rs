//! Chat, damage, sound and clock events. They arrive asynchronously through the Azalea event
//! channel and are drained into the next snapshot.

use azalea::{
    core::{data_registry::DataRegistryWithKey, entity_id::MinecraftEntityId},
    protocol::packets::game::ClientboundGamePacket as Pkt,
    registry::{builtin::SoundEvent as Snd, data::WorldClockKey, Holder},
    Client, Event,
};
use reflex_core::{
    math::Vec3,
    snapshot::{SoundEvent, SoundKind},
};

use super::body::vec3;

/// Overworld night, in ticks of the day clock (monsters spawn freely from tick 13000).
const NIGHT: std::ops::Range<u64> = 13_000..23_000;
const DAY_TICKS: u64 = 24_000;

pub struct RawChat {
    pub sender: String,
    pub text: String,
}

pub struct RawDamage {
    pub source_pos: Option<Vec3>,
    pub attacker: Option<u32>,
}

#[derive(Default)]
pub struct EventBuffer {
    pub chat: Vec<RawChat>,
    pub damage: Vec<RawDamage>,
    pub sounds: Vec<SoundEvent>,
    /// Last known overworld day clock; `None` until the server sends a time packet.
    pub day_time: Option<u64>,
}

impl EventBuffer {
    pub fn is_night(&self) -> bool {
        self.day_time
            .is_some_and(|t| NIGHT.contains(&(t % DAY_TICKS)))
    }

    pub fn on_event(&mut self, bot: &Client, event: &Event) {
        match event {
            Event::Chat(m) => {
                // System lines with no recognisable sender (tellraw, death and join
                // messages) are kept with an empty sender.
                let (sender, text) = m.split_sender_and_content();
                let sender = sender.unwrap_or_default();
                if sender != bot.username() {
                    self.chat.push(RawChat { sender, text });
                }
            }
            Event::Packet(p) => self.on_packet(bot, p),
            _ => {}
        }
    }

    fn on_packet(&mut self, bot: &Client, packet: &Pkt) {
        match packet {
            Pkt::DamageEvent(d) => {
                let mine = bot.component::<MinecraftEntityId>().ok().map(|i| *i);
                if mine == Some(d.entity_id) {
                    self.damage.push(RawDamage {
                        source_pos: d.source_position.map(vec3),
                        attacker: d.source_cause_id.0.map(|i| i.0 as u32),
                    });
                }
            }
            Pkt::Sound(s) => {
                if let Holder::Reference(snd) = &s.sound {
                    if let Some(kind) = sound_kind(*snd) {
                        let pos = Vec3::new(s.x as f64 / 8.0, s.y as f64 / 8.0, s.z as f64 / 8.0);
                        self.sounds.push(SoundEvent { kind, pos });
                    }
                }
            }
            Pkt::Explode(e) => {
                self.sounds.push(SoundEvent {
                    kind: SoundKind::Explosion,
                    pos: vec3(e.center),
                });
            }
            Pkt::SetTime(t) => {
                for (clock, state) in &t.clock_updates {
                    let key = bot.with_registry_holder(|r| clock.key_owned(r));
                    if let Ok(Some(WorldClockKey::Overworld)) = key {
                        self.day_time = Some(state.total_ticks);
                    }
                }
            }
            _ => {}
        }
    }
}

/// BowDraw is not populated: vanilla has no draw sound, only the release sound.
fn sound_kind(s: Snd) -> Option<SoundKind> {
    Some(match s {
        Snd::EntityCreeperPrimed => SoundKind::CreeperPrimed,
        Snd::EntityTntPrimed => SoundKind::TntPrimed,
        Snd::EntityGenericExplode => SoundKind::Explosion,
        Snd::EntityPlayerHurt => SoundKind::PlayerHurt,
        Snd::EntityZombieStep | Snd::EntitySkeletonStep | Snd::EntitySpiderStep => {
            SoundKind::HostileStep
        }
        _ => return None,
    })
}
