//! Open goal vocabulary: a goal name plus one small parameter object. Which names the
//! executor can run today is decided in `bridge::goals`.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalName {
    /// Ends the active goal; the next queued goal resumes.
    End,
    Follow,
    GoTo,
    ComeHere,
    Stay,
    Mine,
    Collect,
    Attack,
    Guard,
    Craft,
    Give,
    Place,
    /// Break the one block at `pos`: the explicit removal, even of a block the body placed.
    Break,
    SleepInBed,
    EatNow,
}

impl GoalName {
    pub const ALL: [GoalName; 15] = [
        GoalName::End,
        GoalName::Follow,
        GoalName::GoTo,
        GoalName::ComeHere,
        GoalName::Stay,
        GoalName::Mine,
        GoalName::Collect,
        GoalName::Attack,
        GoalName::Guard,
        GoalName::Craft,
        GoalName::Give,
        GoalName::Place,
        GoalName::Break,
        GoalName::SleepInBed,
        GoalName::EatNow,
    ];

    pub fn label(self) -> &'static str {
        match self {
            GoalName::End => "end",
            GoalName::Follow => "follow",
            GoalName::GoTo => "go_to",
            GoalName::ComeHere => "come_here",
            GoalName::Stay => "stay",
            GoalName::Mine => "mine",
            GoalName::Collect => "collect",
            GoalName::Attack => "attack",
            GoalName::Guard => "guard",
            GoalName::Craft => "craft",
            GoalName::Give => "give",
            GoalName::Place => "place",
            GoalName::Break => "break",
            GoalName::SleepInBed => "sleep_in_bed",
            GoalName::EatNow => "eat_now",
        }
    }

    /// Parameters a goal cannot do without.
    pub fn required(self) -> &'static [&'static str] {
        match self {
            GoalName::GoTo | GoalName::Break => &["pos"],
            GoalName::Mine | GoalName::Collect | GoalName::Craft => &["kind"],
            GoalName::Place => &["kind", "pos"],
            GoalName::Attack => &["kind"],
            GoalName::Give => &["kind"],
            _ => &[],
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalParams {
    /// Player name; defaults to the configured companion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companion: Option<String>,
    /// Blocks: follow distance, guard radius.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<[f64; 3]>,
    /// place: the box's second corner; every open cell from `pos` to `to` is filled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<[f64; 3]>,
    /// Block, item or entity kind, e.g. "iron_ore", "oak_log", "zombie".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    /// place: clear a block the body placed, or a protected cell, that already holds the
    /// target cell (otherwise such a placement is refused).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replace: Option<bool>,
}

impl GoalParams {
    fn has(&self, field: &str) -> bool {
        match field {
            "companion" => self.companion.is_some(),
            "distance" => self.distance.is_some(),
            "pos" => self.pos.is_some(),
            "to" => self.to.is_some(),
            "kind" => self.kind.as_deref().is_some_and(|k| !k.trim().is_empty()),
            "count" => self.count.is_some(),
            "replace" => self.replace.is_some(),
            _ => false,
        }
    }

    pub fn check(&self, goal: GoalName) -> Result<(), String> {
        if let Some(missing) = goal.required().iter().find(|f| !self.has(f)) {
            return Err(format!("goal {} needs param '{missing}'", goal.label()));
        }
        for (name, p) in [("pos", self.pos), ("to", self.to)] {
            if p.is_some_and(|p| p.iter().any(|v| !v.is_finite())) {
                return Err(format!("{name} must be three finite numbers"));
            }
        }
        if self.distance.is_some_and(|d| !(d.is_finite() && d > 0.0)) {
            return Err("distance must be a positive number".into());
        }
        Ok(())
    }

    /// Canonical key=value form of the request, e.g. `mine kind=iron_ore count=3`, for logs.
    pub fn spec(&self, goal: GoalName) -> String {
        let mut s = goal.label().to_string();
        if let Some(c) = &self.companion {
            s.push_str(&format!(" companion={c}"));
        }
        if let Some(d) = self.distance {
            s.push_str(&format!(" distance={d}"));
        }
        if let Some([x, y, z]) = self.pos {
            s.push_str(&format!(" pos={x},{y},{z}"));
        }
        if let Some([x, y, z]) = self.to {
            s.push_str(&format!(" to={x},{y},{z}"));
        }
        if let Some(k) = &self.kind {
            s.push_str(&format!(" kind={}", k.trim()));
        }
        if let Some(n) = self.count {
            s.push_str(&format!(" count={n}"));
        }
        if self.replace == Some(true) {
            s.push_str(" replace");
        }
        s
    }
}
