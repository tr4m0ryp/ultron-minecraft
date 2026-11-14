use azalea::{Client, SprintDirection, WalkDirection};
use reflex_core::MotorOutput;

pub fn apply(bot: &Client, out: &MotorOutput) {
    // Each setter fails only while the client has no local player entity; the loop does
    // not run in that state, so a failure here is safe to drop.
    let _ = bot.set_direction(out.yaw, out.pitch);
    let _ = bot.set_jumping(out.jump);
    let _ = bot.set_crouching(out.sneak);

    // Sprinting is only possible while moving forward.
    if out.sprint && out.forward > 0 && !out.sneak {
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
