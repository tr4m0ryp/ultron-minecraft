//! Per-tick debug trace, enabled by `MOTOR_TRACE=1`: the snapshot vitals, the owner of each
//! body channel, and every event the executor emitted this tick (including Log class).

use std::time::{SystemTime, UNIX_EPOCH};

use reflex_core::{Channel, Executor, Snapshot};

pub struct Trace {
    on: bool,
}

impl Default for Trace {
    fn default() -> Self {
        Self {
            on: std::env::var_os("MOTOR_TRACE").is_some_and(|v| v != "0"),
        }
    }
}

impl Trace {
    pub fn tick(&self, snap: &Snapshot, exec: &Executor) {
        if !self.on {
            return;
        }
        let me = &snap.me;
        let held = me.hotbar[me.selected_slot as usize % 9]
            .as_ref()
            .map_or("empty".to_string(), |s| format!("{:?}x{}", s.kind, s.count));
        let own = |c| exec.owner(c).unwrap_or("-");
        println!(
            "T {} ms={} pos={:.2},{:.2},{:.2} hp={:.1} food={} sat={:.1} air={} fire={} \
             water={} ground={} slot={} held={} ents={} legs={} gaze={} main={} inv={}",
            snap.tick,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_millis()),
            me.pos.x,
            me.pos.y,
            me.pos.z,
            me.health,
            me.food,
            me.saturation,
            me.air,
            me.on_fire as u8,
            me.in_water as u8,
            me.on_ground as u8,
            me.selected_slot,
            held,
            snap.entities.len(),
            own(Channel::Legs),
            own(Channel::Gaze),
            own(Channel::Main),
            own(Channel::Inv),
        );
        for c in &snap.chat {
            println!(
                "C {} sender={} pos={:?} text={}",
                snap.tick, c.sender, c.sender_pos, c.text
            );
        }
        for d in &snap.damage {
            println!(
                "D {} amount={:.2} attacker={:?} src={:?}",
                snap.tick, d.amount, d.attacker, d.source_pos
            );
        }
        for e in exec.upward.log().rev().take_while(|e| e.tick == snap.tick) {
            println!(
                "E {} {:?} {} {} {}",
                e.tick, e.class, e.kind, e.subject, e.text
            );
        }
    }
}
