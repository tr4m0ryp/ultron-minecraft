use crate::math::Vec3;

#[derive(Clone, Debug, PartialEq)]
pub struct Goal {
    pub id: u64,
    pub spec: String,
    /// Salience the brain assigns, 0..=1, used as the goal behaviour's urgency.
    pub priority: f32,
    /// Where the goal wants the legs to go; `None` for goals the planner resolves later.
    pub target: Option<Vec3>,
    /// Solo goals suspend the companion leash (H8).
    pub solo: bool,
}
