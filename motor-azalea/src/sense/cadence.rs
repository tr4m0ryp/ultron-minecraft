//! Per-entity timing that one tick of metadata does not carry: how long an archer has been
//! drawing, when it last loosed an arrow, and how far a creeper's fuse has burned.

use std::collections::HashMap;

use reflex_core::snapshot::{Entity, EntityKind};

/// A draw at least this long that ends counts as a shot (a skeleton looses at 20 ticks; a
/// shorter draw that ends was abandoned, for example when it lost sight of us).
const SHOT_DRAW: u64 = 18;
/// Creeper fuse length in ticks.
const FUSE: u8 = 30;

#[derive(Clone, Copy, Default)]
struct Track {
    drawing_since: Option<u64>,
    last_shot: Option<u64>,
    swell: u8,
}

/// Tracks entities seen this tick; one that leaves the sensing radius is forgotten.
#[derive(Default)]
pub struct Cadence {
    tracks: HashMap<u32, Track>,
}

impl Cadence {
    pub fn update(&mut self, tick: u64, entities: &mut [Entity]) {
        let mut next = HashMap::with_capacity(entities.len());
        for e in entities.iter_mut() {
            let mut t = self.tracks.get(&e.id).copied().unwrap_or_default();
            if e.kind.is_ranged() {
                match (e.state.drawing_bow, t.drawing_since) {
                    (true, None) => t.drawing_since = Some(tick),
                    (false, Some(since)) => {
                        if tick - since >= SHOT_DRAW {
                            t.last_shot = Some(tick);
                        }
                        t.drawing_since = None;
                    }
                    _ => {}
                }
                let span = |since: u64| (tick - since).min(u16::MAX as u64) as u16;
                e.state.draw_ticks = t.drawing_since.map_or(0, |s| span(s) + 1);
                e.state.since_shot = t.last_shot.map(span);
            } else if e.kind == EntityKind::Creeper {
                t.swell = if e.state.fuse_lit {
                    (t.swell + 1).min(FUSE)
                } else {
                    t.swell.saturating_sub(1)
                };
                e.state.swell = t.swell;
            } else {
                continue;
            }
            next.insert(e.id, t);
        }
        self.tracks = next;
    }
}
