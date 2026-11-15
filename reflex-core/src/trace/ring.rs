use super::goal::GoalTick;

/// Ticks kept (20 s at 20 ticks per second).
pub const RING_TICKS: usize = 400;
/// Reflex vetoes recorded per tick; the rest are dropped from the record.
pub const VETO_SLOTS: usize = 4;

/// Selector decisions of one tick: who owned each body channel, the legs bid's note, the
/// reflexes whose veto was applied, and the goal's state. Two ticks with equal decisions
/// collapse into one run when rendered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Decision {
    /// Owners of Legs, Gaze, Main, Off, Inv (empty string: nobody).
    pub owners: [&'static str; 5],
    pub legs_note: &'static str,
    pub vetoes: [&'static str; VETO_SLOTS],
    pub goal: GoalTick,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TickRec {
    pub tick: u64,
    pub pos: [f32; 3],
    pub d: Decision,
}

/// Fixed-size ring of the newest ticks; pushing never allocates.
#[derive(Debug)]
pub struct TickRing {
    buf: Box<[TickRec]>,
    next: usize,
    len: usize,
}

impl Default for TickRing {
    fn default() -> Self {
        Self::with_capacity(RING_TICKS)
    }
}

impl TickRing {
    pub fn with_capacity(n: usize) -> Self {
        Self {
            buf: vec![TickRec::default(); n.max(1)].into_boxed_slice(),
            next: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, rec: TickRec) {
        self.buf[self.next] = rec;
        self.next = (self.next + 1) % self.buf.len();
        self.len = (self.len + 1).min(self.buf.len());
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// The ring as `TRACE` lines, run-length encoded, headed by `reason`.
    pub fn render(&self, reason: &str) -> Vec<String> {
        super::render::render(self, reason)
    }

    /// Oldest first.
    pub fn iter(&self) -> impl Iterator<Item = &TickRec> {
        let cap = self.buf.len();
        let start = (self.next + cap - self.len) % cap;
        (0..self.len).map(move |i| &self.buf[(start + i) % cap])
    }
}
