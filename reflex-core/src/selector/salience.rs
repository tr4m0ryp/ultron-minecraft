use crate::behaviour::Priority;

/// Result of s_b = class_base(P) + g_b * u_b * (1 - i_b).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Salience {
    pub value: f32,
    /// The gain term g * u * (1 - i); zero means the bid is fully suppressed.
    pub drive: f32,
    /// Inhibition was present but ignored because the bid is P0 (lethal floor).
    pub overridden: bool,
}

/// The gain term is capped below one so a bid never climbs out of its priority class.
pub fn salience(priority: Priority, gain: f32, urgency: f32, inhibition: f32) -> Salience {
    let overridden = priority == Priority::P0 && inhibition > 0.0;
    let i = if priority == Priority::P0 {
        0.0
    } else {
        inhibition.clamp(0.0, 1.0)
    };
    let drive = (gain.max(0.0) * urgency.clamp(0.0, 1.0) * (1.0 - i)).min(0.999);
    Salience {
        value: priority.base() + drive,
        drive,
        overridden,
    }
}
