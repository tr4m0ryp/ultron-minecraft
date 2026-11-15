//! Brewing stands and smithing tables in the fake world. A stand brews at once when it
//! holds bottles, a reagent and fuel (one blaze powder gives 20 batches); a smithing table
//! hands over the result of the three items put in.

use super::state::FakeWorldState;
use crate::crafting::{Shape, BLAZE_POWDER, BREWS_PER_POWDER};
use crate::goals::ports::{MenuStatus, StationJob, StationMenu};
use crate::math::BlockPos;

/// What a fake brewing stand holds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stand {
    pub bottles: Vec<String>,
    pub reagent: Option<String>,
    /// Blaze powder waiting in the fuel slot.
    pub powder: u32,
    /// Batches the burning powder still covers.
    pub charges: u32,
}

const REAGENT: usize = 3;

impl FakeWorldState {
    pub(super) fn station_status(&mut self) -> MenuStatus {
        if let Some((pos, job, left)) = self.station_job.take() {
            if left > 0 {
                self.station_job = Some((pos, job, left - 1));
            } else {
                let done = match job.menu {
                    StationMenu::BrewingStand => self.visit_stand(pos, &job),
                    StationMenu::Smithing => self.visit_smithing(&job),
                };
                self.station_status = Some(match done {
                    Ok(()) => MenuStatus::Done,
                    Err(e) => MenuStatus::Failed(e),
                });
            }
        }
        self.station_status.clone().unwrap_or(MenuStatus::Idle)
    }

    fn take_exact(&mut self, item: &str, n: u32) -> Result<(), String> {
        let have = self.inventory.iter().find(|(k, _)| k == item);
        if have.is_none_or(|(_, c)| *c < n) {
            return Err(format!("ran out of {item}"));
        }
        for (k, c) in self.inventory.iter_mut() {
            if k == item {
                *c -= n;
            }
        }
        self.inventory.retain(|(_, c)| *c > 0);
        Ok(())
    }

    fn visit_stand(&mut self, pos: BlockPos, job: &StationJob) -> Result<(), String> {
        let mut stand = self.stands.remove(&pos).unwrap_or_default();
        if job.take.iter().any(|s| *s < REAGENT) {
            for b in std::mem::take(&mut stand.bottles) {
                self.add_item(&b, 1);
            }
        }
        if job.take.contains(&REAGENT) {
            if let Some(r) = stand.reagent.take() {
                self.add_item(&r, 1);
            }
        }
        for (slot, item, n) in &job.put {
            self.take_exact(item, *n)?;
            if *slot == REAGENT {
                stand.reagent = Some(item.clone());
            } else {
                stand.bottles.push(item.clone());
            }
        }
        if job.fuel.is_some() && stand.powder == 0 && self.take_exact(BLAZE_POWDER, 1).is_ok() {
            stand.powder = 1;
        }
        self.brew(&mut stand);
        self.stands.insert(pos, stand);
        Ok(())
    }

    /// One batch, as if the brewing time had passed.
    fn brew(&self, stand: &mut Stand) {
        let Some(reagent) = stand.reagent.clone() else {
            return;
        };
        let result = |bottle: &str| {
            self.book.all().find_map(|r| match &r.shape {
                Shape::Brew { input, reagent: with }
                    if input.accepts(bottle) && with.accepts(&reagent) =>
                {
                    Some(r.result.clone())
                }
                _ => None,
            })
        };
        if !stand.bottles.iter().any(|b| result(b).is_some()) {
            return;
        }
        if stand.charges == 0 {
            if stand.powder == 0 {
                return;
            }
            stand.powder -= 1;
            stand.charges = BREWS_PER_POWDER;
        }
        stand.charges -= 1;
        stand.reagent = None;
        for b in stand.bottles.iter_mut() {
            if let Some(r) = result(b) {
                *b = r;
            }
        }
    }

    fn visit_smithing(&mut self, job: &StationJob) -> Result<(), String> {
        let item = |slot: usize| {
            job.put
                .iter()
                .find(|(s, _, _)| *s == slot)
                .map(|(_, i, _)| i.clone())
                .unwrap_or_default()
        };
        let (t, b, a) = (item(0), item(1), item(2));
        let result = self.book.all().find_map(|r| match &r.shape {
            Shape::Smith {
                template,
                base,
                addition,
            } if template.accepts(&t) && base.accepts(&b) && addition.accepts(&a) => {
                Some(r.result.clone())
            }
            _ => None,
        });
        let Some(result) = result.filter(|_| !job.result.is_empty()) else {
            return Err("the smithing table showed no result".into());
        };
        for i in [&t, &b, &a] {
            self.take_exact(i, 1)?;
        }
        self.add_item(&result, 1);
        Ok(())
    }
}
