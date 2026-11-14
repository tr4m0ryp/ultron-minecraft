use std::collections::HashMap;

use crate::behaviour::Priority;
use crate::channels::{Channel, ChannelSet, Commands};

/// A challenger must beat the current holder by this much to take its channel.
pub const HYSTERESIS: f32 = 0.1;

/// A bid after salience scoring, ready for allocation.
#[derive(Clone, Debug)]
pub struct Candidate {
    pub id: String,
    pub priority: Priority,
    pub salience: f32,
    pub commands: Commands,
    pub degraded: Option<Commands>,
    pub lock_ticks: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Allocation {
    /// Channel -> index into the candidate slice.
    pub owners: HashMap<Channel, usize>,
    /// The merged plan, one command per owned channel.
    pub plan: Commands,
    /// Candidate indices that won at least one channel.
    pub winners: Vec<usize>,
}

impl Allocation {
    pub fn owner_of(&self, c: Channel) -> Option<usize> {
        self.owners.get(&c).copied()
    }
}

#[derive(Clone, Copy, Debug)]
struct Lock {
    until: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Selector {
    /// Last tick's owner per channel.
    holders: HashMap<Channel, String>,
    locks: HashMap<(Channel, String), Lock>,
}

impl Selector {
    pub fn holder(&self, c: Channel) -> Option<&str> {
        self.holders.get(&c).map(String::as_str)
    }

    fn locked_by(&self, c: Channel, now: u64) -> Option<&str> {
        self.locks
            .iter()
            .find(|((ch, _), l)| *ch == c && now < l.until)
            .map(|((_, id), _)| id.as_str())
    }

    pub fn select(&mut self, cands: &[Candidate], now: u64) -> Allocation {
        self.locks.retain(|_, l| now < l.until);
        // A lock lapses as soon as its owner stops bidding (the atomic action finished).
        self.locks.retain(|(_, id), _| cands.iter().any(|c| &c.id == id));

        let mut order: Vec<usize> = (0..cands.len()).collect();
        order.sort_by(|&a, &b| cands[b].salience.total_cmp(&cands[a].salience));

        let mut alloc = Allocation::default();
        let mut taken: HashMap<Channel, usize> = HashMap::new();
        for respect_reservations in [true, false] {
            for &i in &order {
                if alloc.winners.contains(&i) {
                    continue;
                }
                let variants = [Some(&cands[i].commands), cands[i].degraded.as_ref()];
                for cmds in variants.into_iter().flatten() {
                    let set = cmds.channels();
                    if set.is_empty() || !self.free(cands, i, set, &taken, respect_reservations, now) {
                        continue;
                    }
                    for c in set.iter() {
                        taken.insert(c, i);
                        cmds.copy_channel(c, &mut alloc.plan);
                    }
                    alloc.winners.push(i);
                    break;
                }
            }
        }

        for (&c, &i) in &taken {
            let cand = &cands[i];
            let starts = self.holders.get(&c) != Some(&cand.id);
            if cand.lock_ticks > 0 && starts {
                self.locks.insert((c, cand.id.clone()), Lock { until: now + cand.lock_ticks as u64 });
            }
        }
        self.holders = taken.iter().map(|(&c, &i)| (c, cands[i].id.clone())).collect();
        alloc.owners = taken;
        alloc
    }

    fn free(
        &self,
        cands: &[Candidate],
        i: usize,
        set: ChannelSet,
        taken: &HashMap<Channel, usize>,
        respect_reservations: bool,
        now: u64,
    ) -> bool {
        let me = &cands[i];
        set.iter().all(|c| {
            if taken.contains_key(&c) {
                return false;
            }
            if let Some(owner) = self.locked_by(c, now) {
                if owner != me.id && me.priority != Priority::P0 {
                    return false;
                }
            }
            if !respect_reservations {
                return true;
            }
            match self.holders.get(&c) {
                Some(h) if *h != me.id => !cands.iter().any(|o| {
                    &o.id == h
                        && o.commands.channels().contains(c)
                        && me.salience <= o.salience + HYSTERESIS
                }),
                _ => true,
            }
        })
    }
}
