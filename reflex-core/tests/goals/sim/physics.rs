//! Applying one tick of motor output to the arena snapshot and the fake world.

use reflex_core::channels::{InvCmd, MainCmd};
use reflex_core::goals::items::replaceable;
use reflex_core::goals::ItemDrop;
use reflex_core::math::look_vector;
use reflex_core::snapshot::{BlockKind, ItemKind};
use reflex_core::{BlockPos, MotorOutput, Vec3};

use super::Sim;

impl Sim {
    pub(super) fn apply(&mut self, out: &MotorOutput) {
        let me = &mut self.snap.me;
        me.yaw = out.yaw;
        me.pitch = out.pitch;
        let f = look_vector(out.yaw, 0.0);
        let right = Vec3::new(-f.z, 0.0, f.x);
        let dir = (f * out.forward as f64 + right * out.strafe as f64).normalized();
        let speed = if out.sprint { 0.28 } else { 0.216 };
        let next = me.pos + dir * speed;
        let cell = next.block();
        if self.snap.blocks.get(cell).passable() && self.snap.blocks.get(cell.above()).passable() {
            me.vel = next - me.pos;
            me.pos = next;
        } else {
            me.vel = Vec3::ZERO;
        }
        me.attack_ready = (me.attack_ready + 0.08).min(1.0);
        self.apply_main(out.main);
        if let Some(InvCmd::ToHotbar { from, to }) = out.inv {
            self.move_to_hotbar(from, to);
        }
        let pos = self.snap.me.pos;
        let mut w = self.world.state();
        let (near, far): (Vec<_>, Vec<_>) = w.drops.drain(..).partition(|d| d.pos.dist(pos) <= 1.2);
        w.drops = far;
        for d in near {
            w.add_item(&d.item, d.count);
        }
    }

    fn apply_main(&mut self, cmd: Option<MainCmd>) {
        if !matches!(cmd, Some(MainCmd::Dig(_))) {
            self.dig = None;
        }
        if cmd != Some(MainCmd::Use) {
            self.eat_ticks = 0;
            self.snap.me.using_item = false;
        }
        match cmd {
            Some(MainCmd::SelectSlot(s)) => self.snap.me.selected_slot = s,
            Some(MainCmd::Dig(p)) => {
                let n = match self.dig {
                    Some((q, n)) if q == p => n + 1,
                    _ => 1,
                };
                self.dig = Some((p, n));
                if n >= self.dig_ticks {
                    let mut w = self.world.state();
                    if let Some(name) = w.blocks.remove(&p) {
                        let name = reflex_core::goals::travel::route::drop_of(&name).to_string();
                        match self.drop_offset {
                            Some(off) => {
                                let id = 500 + w.drops.len() as u32;
                                w.drops.push(ItemDrop {
                                    entity_id: id,
                                    item: name,
                                    count: 1,
                                    pos: p.feet_centre() + off,
                                });
                            }
                            None => w.add_item(&name, 1),
                        }
                    }
                    self.snap.blocks.set(p, BlockKind::Air);
                    self.dig = None;
                }
            }
            Some(MainCmd::Attack(id)) => {
                self.attacked.push(id);
                self.snap.me.attack_ready = 0.0;
                let h = self.hits.entry(id).or_insert(0);
                *h += 1;
                if *h >= self.hits_to_kill {
                    self.snap.entities.retain(|e| e.id != id);
                }
            }
            Some(MainCmd::Use) => {
                self.eat_ticks += 1;
                self.snap.me.using_item = true;
                if self.eat_ticks == 32 {
                    let slot = self.snap.me.selected_slot as usize;
                    if let Some(s) = self.snap.me.hotbar[slot].as_mut() {
                        if let ItemKind::Food { nutrition, .. } = s.kind {
                            self.snap.me.food = (self.snap.me.food + nutrition).min(20);
                        }
                        s.count -= 1;
                    }
                }
            }
            Some(MainCmd::PlaceOn(support)) => self.place(support),
            Some(MainCmd::PlaceAt { cell, .. }) => self.place_into(cell),
            _ => {}
        }
    }

    fn place(&mut self, support: BlockPos) {
        let mut w = self.world.state();
        if w.blocks.get(&support).is_some_and(|n| n.ends_with("bed")) {
            w.sleeping = true;
            return;
        }
        let on_top = self.place_on_top.then(|| support.above());
        let Some(at) = on_top.or(self.place_at) else {
            return;
        };
        drop(w);
        self.place_into(at);
    }

    fn place_into(&mut self, at: BlockPos) {
        let mut w = self.world.state();
        if w.blocks.get(&at).is_some_and(|n| !replaceable(n)) {
            return;
        }
        let sel = self.snap.me.selected_slot;
        let Some(name) = w
            .slots
            .iter()
            .find(|(_, s)| **s == sel)
            .map(|(n, _)| n.clone())
        else {
            return;
        };
        if w.take_item(&name, 1) == 1 {
            w.blocks.insert(at, name);
            self.snap.blocks.set(at, BlockKind::Solid);
        }
    }

    fn move_to_hotbar(&mut self, from: u8, to: u8) {
        let me = &mut self.snap.me;
        let i = me.inventory.iter().position(|(s, _)| *s == from);
        let moved = i.map(|i| me.inventory.remove(i).1);
        if let Some(old) = me.hotbar[to as usize].take() {
            me.inventory.push((from, old));
        }
        me.hotbar[to as usize] = moved;
        let mut w = self.world.state();
        for s in w.slots.values_mut() {
            if *s == from {
                *s = to;
            } else if *s == to {
                *s = from;
            }
        }
    }
}
