//! The body's state at a decision, taken from the tick's snapshot before the decision is
//! applied: vitals, position, time of day, entities, companion, the whole inventory, the
//! running and queued goals with progress, and the last events.

use std::collections::BTreeMap;

use reflex_core::events::EventClass;
use reflex_core::Snapshot;
use serde::{Deserialize, Serialize};

use crate::host::Host;
use crate::wire::mode_label;

/// Entities listed, nearest first.
pub const NEAR_MAX: usize = 24;
/// Entities farther than this are left out.
const NEAR_RADIUS: f64 = 48.0;
/// Wake and digest events listed, newest last.
pub const EVENTS_KEPT: usize = 12;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EntityView {
    pub kind: String,
    pub id: u32,
    pub dist: f64,
    pub pos: [f64; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Health, known for the companion only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hp: Option<f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GoalLine {
    pub id: u64,
    pub spec: String,
    pub priority: f32,
    /// running, blocked or preempted; absent for a queued goal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// Units done and total, for counted goals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<(u32, u32)>,
    /// The latest progress note or blocked reason: the plan step under way.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Goals {
    pub running: Option<GoalLine>,
    /// Suspended goals, the next to resume last.
    pub queued: Vec<GoalLine>,
    /// Outcome of the last goal that ended, e.g. `#3 mine done 3/3: mined 3 stone`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventLine {
    pub t: u64,
    /// W wake, D digest.
    pub class: char,
    pub kind: String,
    pub subject: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct State {
    pub hp: f32,
    pub food: u8,
    pub saturation: f32,
    pub air: i32,
    pub pos: [f64; 3],
    pub on_ground: bool,
    pub in_water: bool,
    pub on_fire: bool,
    /// Time of day in ticks, 0..24000; absent until the clock is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day_time: Option<u32>,
    pub night: bool,
    pub mode: String,
    /// The behaviour that holds the legs (`GOAL`, a reflex id), when one does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legs: Option<String>,
    /// Selected hotbar slot, 0..=8.
    pub slot: u8,
    pub entities: Vec<EntityView>,
    /// Entities in range beyond the `NEAR_MAX` listed.
    pub entities_more: usize,
    pub companion: Option<EntityView>,
    /// Item name to count over the main inventory and hotbar.
    pub inventory: BTreeMap<String, u32>,
    pub goals: Goals,
    pub events: Vec<EventLine>,
}

fn r2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

fn xyz(p: reflex_core::Vec3) -> [f64; 3] {
    [r2(p.x), r2(p.y), r2(p.z)]
}

impl State {
    /// The state as `snap` and the host show it now.
    pub fn take<H: Host>(host: &mut H, snap: &Snapshot) -> Self {
        let me = &snap.me;
        let mut near: Vec<EntityView> = snap
            .entities
            .iter()
            .map(|e| EntityView {
                kind: e.kind.label().into(),
                id: e.id,
                dist: r2(e.pos.dist(me.pos)),
                pos: xyz(e.pos),
                name: e.name.clone(),
                hp: None,
            })
            .filter(|e| e.dist <= NEAR_RADIUS)
            .collect();
        near.sort_by(|a, b| a.dist.total_cmp(&b.dist));
        let entities_more = near.len().saturating_sub(NEAR_MAX);
        near.truncate(NEAR_MAX);
        let status = host.goal_status();
        let legs = host.legs_owner();
        let inventory = host.inventory().into_iter().fold(
            BTreeMap::new(),
            |mut m: BTreeMap<String, u32>, (name, n)| {
                *m.entry(name).or_default() += n;
                m
            },
        );
        let mut events: Vec<EventLine> = host
            .upward()
            .log()
            .rev()
            .filter(|e| e.class != EventClass::Log)
            .take(EVENTS_KEPT)
            .map(|e| EventLine {
                t: e.tick,
                class: if e.class == EventClass::Wake {
                    'W'
                } else {
                    'D'
                },
                kind: e.kind.clone(),
                subject: e.subject.clone(),
                text: e.text.clone(),
            })
            .collect();
        events.reverse();
        let knobs = host.knobs();
        let line = |g: &reflex_core::knobs::Goal| GoalLine {
            id: g.id,
            spec: g.spec.clone(),
            priority: g.priority,
            state: None,
            progress: None,
            note: None,
        };
        let running = knobs.goal.as_ref().map(|g| {
            let live = status
                .as_ref()
                .filter(|s| s.goal_id == g.id && !s.state.is_final());
            GoalLine {
                state: Some(live.map_or("running", |s| s.state.label()).into()),
                progress: live.and_then(|s| s.progress),
                note: live.and_then(|s| s.note.clone()),
                ..line(g)
            }
        });
        State {
            hp: me.health,
            food: me.food,
            saturation: me.saturation,
            air: me.air,
            pos: xyz(me.pos),
            on_ground: me.on_ground,
            in_water: me.in_water,
            on_fire: me.on_fire,
            day_time: snap.day_time,
            night: snap.is_night,
            mode: mode_label(knobs.mode).into(),
            legs,
            slot: me.selected_slot,
            entities: near,
            entities_more,
            companion: snap.companion.as_ref().map(|c| EntityView {
                kind: "player".into(),
                id: c.entity_id.unwrap_or(0),
                dist: r2(c.pos.dist(me.pos)),
                pos: xyz(c.pos),
                name: Some(c.name.clone()),
                hp: c.health,
            }),
            inventory,
            goals: Goals {
                running,
                queued: knobs.goal_queue.iter().map(line).collect(),
                last: status
                    .as_ref()
                    .filter(|s| s.state.is_final())
                    .map(|s| s.line()),
            },
            events,
        }
    }
}
