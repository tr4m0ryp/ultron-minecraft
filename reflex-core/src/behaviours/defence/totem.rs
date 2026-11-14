use crate::behaviour::{Behaviour, Bid, Ctx, Priority};
use crate::channels::{Commands, InvCmd};
use crate::snapshot::{EntityKind, ItemKind};

/// R11: a totem is carried but not in the offhand, and death is plausible: hp 6 or less, a
/// lit creeper fuse within 4 blocks, or a predicted fall that is lethal. Move it to the
/// offhand; after a pop the same trigger re-equips the next one.
#[derive(Default)]
pub struct Totem;

const LOW_HP: f32 = 6.0;
const FUSE_RANGE: f64 = 4.0;

impl Behaviour for Totem {
    fn id(&self) -> &'static str {
        "R11"
    }

    fn evaluate(&mut self, ctx: &Ctx) -> Option<Bid> {
        let snap = ctx.snap;
        let me = &snap.me;
        if me.offhand.is_some_and(|s| s.kind == ItemKind::Totem) {
            return None;
        }
        let slot = me.find_any(|k| k == ItemKind::Totem)?;
        let fuse = !snap
            .nearby(FUSE_RANGE, |e| e.kind == EntityKind::Creeper && e.state.fuse_lit)
            .is_empty();
        let fall = ctx.pred.fall.is_some_and(|f| !f.into_void && f.damage >= me.hp_total());
        if !(me.health <= LOW_HP || fuse || fall) {
            return None;
        }
        let cmd = if slot == me.selected_slot { InvCmd::SwapHands } else { InvCmd::ToOffhand(slot) };
        Some(Bid::new(Priority::P0, 1.0, "totem_equip", Commands::default().inv(cmd)))
    }
}
