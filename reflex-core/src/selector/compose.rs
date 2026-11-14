use crate::channels::{Commands, GazeCmd, InvCmd, MainCmd, OffCmd, Speed};
use crate::math::{look_vector, yaw_pitch_to, Vec3};
use crate::snapshot::Snapshot;

/// Final per-tick player inputs, expressed the way a client applies them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MotorOutput {
    /// -1 back, 0, +1 forward (S/W).
    pub forward: i8,
    /// -1 left, 0, +1 right (A/D).
    pub strafe: i8,
    pub jump: bool,
    pub sneak: bool,
    pub sprint: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub main: Option<MainCmd>,
    pub off: Option<OffCmd>,
    pub inv: Option<InvCmd>,
    pub chat: Option<String>,
}

/// Turns any gaze request into explicit angles. With no gaze owner the head follows the
/// legs' heading, or stays where it is.
pub fn resolve_gaze(snap: &Snapshot, plan: &mut Commands) {
    let me = &snap.me;
    let (yaw, pitch) = match plan.gaze {
        Some(GazeCmd::Angles { yaw, pitch }) => (yaw, pitch),
        Some(GazeCmd::LookAt(p)) => yaw_pitch_to(me.eye(), p),
        None => match plan.legs.map(|l| l.heading.horizontal()) {
            Some(h) if h.len() > 1e-6 => (yaw_pitch_to(Vec3::ZERO, h).0, me.pitch),
            _ => (me.yaw, me.pitch),
        },
    };
    plan.gaze = Some(GazeCmd::Angles {
        yaw,
        pitch: pitch.clamp(-90.0, 90.0),
    });
}

/// Motor composition: the legs' world-frame heading becomes WASD relative to the final yaw.
/// Sprint only applies when moving forward.
pub fn compose(snap: &Snapshot, plan: &Commands) -> MotorOutput {
    let (yaw, pitch) = match plan.gaze {
        Some(GazeCmd::Angles { yaw, pitch }) => (yaw, pitch),
        Some(GazeCmd::LookAt(p)) => yaw_pitch_to(snap.me.eye(), p),
        None => (snap.me.yaw, snap.me.pitch),
    };
    let mut out = MotorOutput {
        yaw,
        pitch,
        main: plan.main,
        off: plan.off,
        inv: plan.inv,
        chat: plan.chat.as_ref().map(|c| c.0.clone()),
        ..Default::default()
    };
    if let Some(legs) = plan.legs {
        let h = legs.heading.horizontal().normalized();
        out.jump = legs.jump;
        out.sneak = legs.speed == Speed::Sneak;
        if legs.speed != Speed::Stop && h.len() > 0.5 {
            let f = look_vector(yaw, 0.0);
            let right = Vec3::new(-f.z, 0.0, f.x);
            out.forward = quantize(h.dot(f));
            out.strafe = quantize(h.dot(right));
        }
        out.sprint = legs.speed == Speed::Sprint && out.forward > 0;
    }
    out
}

/// cos(67.5 deg): an axis is pressed when the heading is within 67.5 degrees of it, giving
/// eight evenly split WASD directions.
fn quantize(v: f64) -> i8 {
    const T: f64 = 0.382_683_4;
    if v > T {
        1
    } else if v < -T {
        -1
    } else {
        0
    }
}
