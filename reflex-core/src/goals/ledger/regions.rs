//! Protected boxes outlive a restart: the knobs hold the live list the routes read, the
//! ledger keeps a copy in its file. Live, a restart used to drop every box thought had
//! marked, so a base protected in one session could be dug in the next.

use super::{Ledger, Region};

impl Ledger {
    /// Mirrors the knobs' protected boxes. The first call restores the remembered boxes into
    /// `live` (keeping any marked before it); later calls record every change of `live`.
    pub fn sync_regions(&self, live: &mut Vec<Region>) {
        let mut i = self.0.borrow_mut();
        if !i.regions_restored {
            i.regions_restored = true;
            for r in i.regions.clone() {
                if !live.contains(&r) {
                    live.push(r);
                }
            }
        }
        if i.regions != *live {
            i.regions = live.clone();
            i.dirty = true;
        }
    }

    /// The protected boxes the ledger holds, for tests and reports.
    pub fn regions(&self) -> Vec<Region> {
        self.0.borrow().regions.clone()
    }
}
