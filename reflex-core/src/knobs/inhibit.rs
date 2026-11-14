use crate::math::{Aabb, Vec3};

#[derive(Clone, Debug, PartialEq)]
pub enum Scope {
    Everywhere,
    /// Applies only while the player stands inside the box.
    Region(Aabb),
    /// Applies until the goal with this id ends.
    UntilGoalEnds(u64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Inhibition {
    pub id: u64,
    pub behaviour: String,
    pub scope: Scope,
    pub strength: f32,
    pub expires_at: Option<u64>,
}

impl Inhibition {
    pub fn expired(&self, now: u64) -> bool {
        self.expires_at.is_some_and(|t| now >= t)
    }

    pub fn active(&self, pos: Vec3, now: u64, goal: Option<u64>) -> bool {
        if self.expired(now) {
            return false;
        }
        match &self.scope {
            Scope::Everywhere => true,
            Scope::Region(b) => b.contains(pos),
            Scope::UntilGoalEnds(id) => goal == Some(*id),
        }
    }
}
