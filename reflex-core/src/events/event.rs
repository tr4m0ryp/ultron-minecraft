use crate::math::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventClass {
    /// Returns the brain's long-poll at once.
    Wake,
    /// Delivered on the brain's next observe call.
    Digest,
    /// Kept for debugging only.
    Log,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub tick: u64,
    pub class: EventClass,
    /// Event type, e.g. "lava_withdraw", "override", "armed_fired".
    pub kind: String,
    /// What it is about, e.g. "creeper", "R5"; empty when not applicable.
    pub subject: String,
    /// Size of the stimulus; a large change dishabituates.
    pub magnitude: f32,
    pub hp: Option<f32>,
    /// Coarse place (16-block cell) for the habituation key.
    pub place: Option<(i32, i32, i32)>,
    /// Self-caused per the efference copy: never a prediction error.
    pub self_caused: bool,
    /// Bypasses the WAKE rate limit (P0 and lethal overrides).
    pub critical: bool,
    pub text: String,
}

impl Event {
    pub fn new(tick: u64, class: EventClass, kind: &str, subject: &str) -> Self {
        Self {
            tick,
            class,
            kind: kind.to_string(),
            subject: subject.to_string(),
            magnitude: 1.0,
            hp: None,
            place: None,
            self_caused: false,
            critical: false,
            text: String::new(),
        }
    }

    pub fn at(mut self, p: Vec3) -> Self {
        self.place = Some((
            (p.x / 16.0).floor() as i32,
            (p.y / 16.0).floor() as i32,
            (p.z / 16.0).floor() as i32,
        ));
        self
    }
    pub fn hp(mut self, hp: f32) -> Self {
        self.hp = Some(hp);
        self
    }
    pub fn magnitude(mut self, m: f32) -> Self {
        self.magnitude = m;
        self
    }
    pub fn critical(mut self) -> Self {
        self.critical = true;
        self
    }
    pub fn self_caused(mut self) -> Self {
        self.self_caused = true;
        self
    }
    pub fn text(mut self, t: impl Into<String>) -> Self {
        self.text = t.into();
        self
    }

    /// Habituation key: type, subject class, coarse place.
    pub fn key(&self) -> String {
        match self.place {
            Some((x, y, z)) => format!("{}|{}|{x},{y},{z}", self.kind, self.subject),
            None => format!("{}|{}", self.kind, self.subject),
        }
    }
}
