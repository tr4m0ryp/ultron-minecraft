use super::gains::Gains;

/// A preset of gains, analogous to a neuromodulatory state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Explore,
    Mine,
    Build,
    Fight,
    Retreat,
    Follow,
    IdleSafe,
}

impl Mode {
    pub fn preset(self) -> Gains {
        let mut g = Gains::default();
        match self {
            Mode::Explore => {}
            Mode::Mine => {
                g.caution = 0.7;
                g.per_behaviour.insert("H8".into(), 0.5);
            }
            Mode::Build => g.caution = 0.7,
            Mode::Fight => {
                g.aggression = 0.85;
                g.caution = 0.3;
            }
            Mode::Retreat => {
                g.aggression = 0.1;
                g.caution = 0.8;
                g.hp_floor = 12.0;
            }
            Mode::Follow => g.leash = 4.0,
            Mode::IdleSafe => {
                g.caution = 0.8;
                g.aggression = 0.3;
            }
        }
        g
    }
}
