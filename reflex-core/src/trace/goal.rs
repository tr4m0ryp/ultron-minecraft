/// What the navigator reported on the goal's last poll.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavCode {
    /// The goal did not poll the navigator this tick.
    #[default]
    None,
    Idle,
    Planning,
    Moving,
    /// Moving with hand work (dig or place) first.
    Work,
    Arrived,
    Unreachable,
    TargetLost,
    Stalled,
    NoScaffold,
}

impl NavCode {
    pub fn label(self) -> &'static str {
        match self {
            NavCode::None => "-",
            NavCode::Idle => "idle",
            NavCode::Planning => "planning",
            NavCode::Moving => "moving",
            NavCode::Work => "work",
            NavCode::Arrived => "arrived",
            NavCode::Unreachable => "unreachable",
            NavCode::TargetLost => "target_lost",
            NavCode::Stalled => "STALLED",
            NavCode::NoScaffold => "no_scaffold",
        }
    }
}

pub const FLAG_JUMP: u8 = 1;
pub const FLAG_SPRINT: u8 = 2;
pub const FLAG_SNEAK: u8 = 4;

/// The goal runner's state for one tick: small, `Copy`, no allocation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GoalTick {
    /// 0 when no goal is active.
    pub goal_id: u64,
    pub label: &'static str,
    /// Note of the bid the goal made this tick (its phase), empty when it did not bid.
    pub phase: &'static str,
    pub nav: NavCode,
    /// Steering flags (jump, sprint, sneak) of the last `Moving` poll.
    pub flags: u8,
    /// Next waypoint block of the last `Moving` poll.
    pub wp: [i32; 3],
    /// The goal bid but a reflex won the legs.
    pub preempted: bool,
    /// The goal's bid asked the legs to move (not a hold).
    pub moving: bool,
    /// The goal reported a blocked reason this tick.
    pub blocked: bool,
}
