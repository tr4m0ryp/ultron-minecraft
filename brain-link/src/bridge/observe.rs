use reflex_core::behaviours::GOAL_ID;
use reflex_core::goals::{GoalState, GoalStatus};
use reflex_core::knobs::Knobs;
use reflex_core::snapshot::{ItemKind, ItemStack};
use reflex_core::Snapshot;

use crate::wire::{mode_label, CompanionView, GoalView, NearGroup, Observation};

/// Entities farther than this are left out of the summary.
const NEAR_RADIUS: f64 = 32.0;
const NEAR_GROUPS_MAX: usize = 8;

pub fn observe(
    knobs: &Knobs,
    legs_owner: Option<&str>,
    status: Option<&GoalStatus>,
    snap: &Snapshot,
    digest: String,
    wakes_pending: usize,
) -> Observation {
    let me = &snap.me;
    let p = me.pos;
    Observation {
        tick: snap.tick,
        pos: [r1(p.x), r1(p.y), r1(p.z)],
        hp: (me.health * 10.0).round() / 10.0,
        food: me.food,
        held: me.held().map(held_label),
        night: snap.is_night,
        mode: mode_label(knobs.mode).into(),
        near: near(snap),
        companion: snap.companion.as_ref().map(|c| CompanionView {
            name: c.name.clone(),
            dist: r1(c.pos.dist(p)),
            hp: c.health,
        }),
        goal: knobs.goal.as_ref().map(|g| {
            let live = status.filter(|s| s.goal_id == g.id && !s.state.is_final());
            GoalView {
                id: g.id,
                spec: g.spec.clone(),
                priority: g.priority,
                status: goal_status(g, live, legs_owner, snap),
                progress: live
                    .and_then(|s| s.progress)
                    .map(|(d, t)| format!("{d}/{t}")),
                note: live.and_then(|s| s.note.clone()),
                queued: knobs.goal_queue.len(),
            }
        }),
        last_goal: status.filter(|s| s.state.is_final()).map(GoalStatus::line),
        digest,
        wakes_pending,
    }
}

fn goal_status(
    g: &reflex_core::knobs::Goal,
    live: Option<&GoalStatus>,
    owner: Option<&str>,
    snap: &Snapshot,
) -> String {
    match (live.map(|s| s.state), owner) {
        (Some(GoalState::Blocked), _) => return "blocked".into(),
        (Some(GoalState::Preempted), Some(other)) if other != GOAL_ID => {
            return format!("paused_by:{other}")
        }
        (Some(GoalState::Preempted), _) => return "preempted".into(),
        (Some(GoalState::Running), _) => return "running".into(),
        _ => {}
    }
    let following = g.target.is_none() && g.spec.starts_with("follow");
    match owner {
        Some(GOAL_ID) => "running".into(),
        Some("H8") if following => "running".into(),
        Some(other) => format!("paused_by:{other}"),
        None => match g.target {
            Some(t) if (t - snap.me.pos).horizontal().len() <= 1.0 => "arrived".into(),
            _ => "idle".into(),
        },
    }
}

fn near(snap: &Snapshot) -> Vec<NearGroup> {
    let me = snap.me.pos;
    let mut groups: Vec<NearGroup> = Vec::new();
    for e in &snap.entities {
        let d = e.pos.dist(me);
        if d > NEAR_RADIUS {
            continue;
        }
        let kind = e.kind.label();
        match groups.iter_mut().find(|g| g.kind == kind) {
            Some(g) => {
                g.n += 1;
                g.dist = g.dist.min(r1(d));
            }
            None => groups.push(NearGroup {
                kind: kind.into(),
                n: 1,
                dist: r1(d),
            }),
        }
    }
    groups.sort_by(|a, b| a.dist.total_cmp(&b.dist));
    groups.truncate(NEAR_GROUPS_MAX);
    groups
}

fn held_label(s: ItemStack) -> String {
    let name = match s.kind {
        ItemKind::Sword => "sword",
        ItemKind::Axe => "axe",
        ItemKind::Pickaxe => "pickaxe",
        ItemKind::Shovel => "shovel",
        ItemKind::Shield => "shield",
        ItemKind::Totem => "totem",
        ItemKind::WaterBucket => "water_bucket",
        ItemKind::EmptyBucket => "bucket",
        ItemKind::GoldenApple => "golden_apple",
        ItemKind::Food { .. } => "food",
        ItemKind::ThrowawayBlock => "block",
        ItemKind::Torch => "torch",
        ItemKind::CarvedPumpkin => "carved_pumpkin",
        ItemKind::Other => "other",
    };
    if s.count > 1 {
        format!("{name} x{}", s.count)
    } else {
        name.into()
    }
}

fn r1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}
