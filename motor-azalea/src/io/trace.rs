//! Per-tick debug trace, enabled by `MOTOR_TRACE=1`: the snapshot vitals, the owner of each
//! body channel, the block the hand works on (`hand=dig:x,y,z` or `place:x,y,z`) and every
//! event the executor emitted this tick (including Log class). On a tick where the owner of
//! any channel changed, `A <tick> us=<n>` follows once the output is written to the
//! connection: microseconds from the start of sensing to the last packet handed over.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reflex_core::{channels::MainCmd, Channel, Executor, MotorOutput, Snapshot};

pub struct Trace {
    on: bool,
    /// Channel owners of the last tick, to tell a tick on which a response begins.
    owners: [Option<&'static str>; 5],
    changed: bool,
}

impl Default for Trace {
    fn default() -> Self {
        Self {
            on: std::env::var_os("MOTOR_TRACE").is_some_and(|v| v != "0"),
            owners: [None; 5],
            changed: false,
        }
    }
}

impl Trace {
    /// After the tick's output went to the connection; `took` counts from the start of
    /// sensing.
    pub fn acted(&mut self, tick: u64, took: Duration) {
        if self.on && std::mem::take(&mut self.changed) {
            println!("A {tick} us={}", took.as_micros());
        }
    }

    pub fn tick(&mut self, snap: &Snapshot, exec: &Executor, out: &MotorOutput, sensed: Duration) {
        if !self.on {
            return;
        }
        let owners =
            [Channel::Legs, Channel::Gaze, Channel::Main, Channel::Off, Channel::Inv].map(|c| exec.owner(c));
        self.changed = owners != self.owners;
        self.owners = owners;
        let me = &snap.me;
        let held = me.hotbar[me.selected_slot as usize % 9]
            .as_ref()
            .map_or("empty".to_string(), |s| format!("{:?}x{}", s.kind, s.count));
        let own = |c| exec.owner(c).unwrap_or("-");
        println!(
            "T {} ms={} pos={:.2},{:.2},{:.2} hp={:.1} food={} sat={:.1} air={} fire={} \
             water={} ground={} slot={} held={} ents={} legs={} gaze={} main={} inv={} \
             vel={:.3},{:.3},{:.3} off={} shield={} near={} note={} hand={}",
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
            me.vel.x,
            me.vel.y,
            me.vel.z,
            own(Channel::Off),
            me.shield_ticks,
            nearest(snap),
            exec.legs_note().unwrap_or("-"),
            hand(out),
        );
        if sensed.as_millis() >= 10 {
            println!("P {} sense_ms={}", snap.tick, sensed.as_millis());
        }
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

/// The block the main hand digs or places this tick.
fn hand(out: &MotorOutput) -> String {
    match out.main {
        Some(MainCmd::Dig(p)) => format!("dig:{},{},{}", p.x, p.y, p.z),
        Some(MainCmd::PlaceAt { cell: p, .. }) => format!("place:{},{},{}", p.x, p.y, p.z),
        Some(MainCmd::PlaceOn(p)) => format!("click:{},{},{}", p.x, p.y, p.z),
        _ => "-".into(),
    }
}

/// Nearest hostile as kind:distance plus its timing state (archer draw ticks or ticks since
/// its shot, creeper fuse progress), for offline analysis of the defensive reflexes.
fn nearest(snap: &Snapshot) -> String {
    let Some(e) = snap.nearest_hostile(32.0) else {
        return "-".into();
    };
    let st = &e.state;
    let timing = if e.kind.is_ranged() {
        match (st.drawing_bow, st.since_shot) {
            (true, _) => format!(":draw{}", st.draw_ticks),
            (false, Some(s)) => format!(":shot{s}"),
            _ => String::new(),
        }
    } else if st.fuse_lit || st.swell > 0 {
        format!(":fuse{}", st.swell)
    } else {
        String::new()
    };
    format!("{}:{:.1}{timing}", e.kind.label(), e.pos.dist(snap.me.pos))
}
