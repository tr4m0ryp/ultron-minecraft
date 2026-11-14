use azalea::{Client, SprintDirection, WalkDirection};
use reflex_core::MotorOutput;

/// `slowed` is the item-use slowdown from `HandState::slowed`.
pub fn apply(bot: &Client, out: &MotorOutput, slowed: Option<bool>) {
    // Each setter fails only while the client has no local player entity; the loop does
    // not run in that state, so a failure here is safe to drop.
    let _ = bot.set_direction(out.yaw, out.pitch);
    let _ = bot.set_jumping(out.jump);
    let _ = bot.set_crouching(out.sneak);

    if slowed == Some(true) {
        bot.walk(WalkDirection::None);
        return;
    }
    // Sprinting is only possible while moving forward, and not while using an item.
    if out.sprint && out.forward > 0 && !out.sneak && slowed.is_none() {
        bot.sprint(match out.strafe.signum() {
            1 => SprintDirection::ForwardRight,
            -1 => SprintDirection::ForwardLeft,
            _ => SprintDirection::Forward,
        });
    } else {
        bot.walk(walk_direction(out.forward.signum(), out.strafe.signum()));
    }
}

fn walk_direction(forward: i8, strafe: i8) -> WalkDirection {
    match (forward, strafe) {
        (1, 0) => WalkDirection::Forward,
        (1, 1) => WalkDirection::ForwardRight,
        (1, -1) => WalkDirection::ForwardLeft,
        (-1, 0) => WalkDirection::Backward,
        (-1, 1) => WalkDirection::BackwardRight,
        (-1, -1) => WalkDirection::BackwardLeft,
        (0, 1) => WalkDirection::Right,
        (0, -1) => WalkDirection::Left,
        _ => WalkDirection::None,
    }
}
