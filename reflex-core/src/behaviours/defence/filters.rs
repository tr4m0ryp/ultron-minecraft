use crate::behaviour::{Behaviour, Bid, Ctx, Priority, Veto};
use crate::channels::{Commands, GazeCmd, MainCmd};
use crate::math::look_vector;
use crate::snapshot::{EntityClass, EntityKind, Snapshot};

const ENDERMAN_RANGE: f64 = 64.0;

/// Vanilla provocation test: the look vector points at the enderman's head within a
/// cone that narrows with distance (dot > 1 - 0.025 / d).
fn provokes(snap: &Snapshot, yaw: f32, pitch: f32) -> bool {
    let eye = snap.me.eye();
    let look = look_vector(yaw, pitch);
    snap.entities.iter().filter(|e| e.kind == EntityKind::Enderman).any(|e| {
        let to = e.head() - eye;
        let d = to.len();
        d <= ENDERMAN_RANGE && d > 1e-6 && look.dot(to.normalized()) > 1.0 - 0.025 / d
    })
}

/// D5 / O7: never look at an enderman's head unless wearing a carved pumpkin. Applied to
/// every final look; tilts the pitch away until the look is safe.
#[derive(Default)]
pub struct EndermanGazeFilter;

impl Behaviour for EndermanGazeFilter {
    fn id(&self) -> &'static str {
        "D5"
    }

    fn evaluate(&mut self, _ctx: &Ctx) -> Option<Bid> {
        None
    }

    fn veto(&mut self, ctx: &Ctx, plan: &mut Commands) -> Option<Veto> {
        if ctx.snap.me.wearing_pumpkin {
            return None;
        }
        let Some(GazeCmd::Angles { yaw, pitch }) = plan.gaze else { return None };
        if !provokes(ctx.snap, yaw, pitch) {
            return None;
        }
        let fixed = [pitch + 25.0, pitch - 25.0, 60.0, -60.0, 90.0]
            .into_iter()
            .map(|p| p.clamp(-90.0, 90.0))
            .find(|&p| !provokes(ctx.snap, yaw, p))
            .unwrap_or(90.0);
        plan.gaze = Some(GazeCmd::Angles { yaw, pitch: fixed });
        Some(Veto { priority: Priority::P1, note: "enderman_gaze_filter" })
    }
}

/// D11: never attack neutral or friendly entities, the companion, or other players.
/// The brain lifts the mask by inhibiting D11.
#[derive(Default)]
pub struct AttackMask;

impl Behaviour for AttackMask {
    fn id(&self) -> &'static str {
        "D11"
    }

    fn evaluate(&mut self, _ctx: &Ctx) -> Option<Bid> {
        None
    }

    fn veto(&mut self, ctx: &Ctx, plan: &mut Commands) -> Option<Veto> {
        let Some(MainCmd::Attack(id)) = plan.main else { return None };
        let companion = ctx.snap.companion.as_ref().and_then(|c| c.entity_id) == Some(id);
        let protected = ctx.snap.entity(id).is_some_and(|e| e.kind.class() == EntityClass::Protected);
        if !(companion || protected) {
            return None;
        }
        plan.main = None;
        Some(Veto { priority: Priority::P1, note: "attack_mask" })
    }
}
