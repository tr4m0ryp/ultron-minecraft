use super::event::Event;

/// Rough token estimate (about four characters per token for this terse format).
pub fn estimate_tokens(s: &str) -> usize {
    s.len().div_ceil(4)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Digest {
    pub text: String,
    pub tokens: usize,
    pub dropped: usize,
}

struct Line {
    first_tick: u64,
    kind: String,
    subject: String,
    count: u32,
    hp_first: Option<f32>,
    hp_last: Option<f32>,
    weight: f32,
}

impl Line {
    fn render(&self, t0: u64) -> String {
        let mut s = format!("t+{} {}", self.first_tick.saturating_sub(t0), self.kind);
        if !self.subject.is_empty() {
            s.push(' ');
            s.push_str(&self.subject);
        }
        s.push_str(&format!(" x{}", self.count));
        if let (Some(a), Some(b)) = (self.hp_first, self.hp_last) {
            if (a - b).abs() >= 0.5 {
                s.push_str(&format!(" hp {a:.0}->{b:.0}"));
            }
        }
        s
    }
}

/// Aggregates events by (kind, subject) with counts and net HP change, then trims the
/// lowest-weight lines until the text fits `cap` tokens.
pub fn build(events: &[(Event, f32)], since: u64, cap: usize) -> Digest {
    let mut lines: Vec<Line> = Vec::new();
    for (e, w) in events {
        match lines
            .iter_mut()
            .find(|l| l.kind == e.kind && l.subject == e.subject)
        {
            Some(l) => {
                l.count += 1;
                l.hp_last = e.hp.or(l.hp_last);
                l.weight += w;
            }
            None => lines.push(Line {
                first_tick: e.tick,
                kind: e.kind.clone(),
                subject: e.subject.clone(),
                count: 1,
                hp_first: e.hp,
                hp_last: e.hp,
                weight: *w,
            }),
        }
    }
    let mut dropped = 0;
    loop {
        let mut text = lines
            .iter()
            .map(|l| l.render(since))
            .collect::<Vec<_>>()
            .join(" | ");
        if dropped > 0 {
            text.push_str(&format!(" | +{dropped} more"));
        }
        let tokens = estimate_tokens(&text);
        if tokens <= cap || lines.is_empty() {
            return Digest {
                text,
                tokens,
                dropped,
            };
        }
        let weakest = lines
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.weight.total_cmp(&b.1.weight))
            .map(|(i, _)| i)
            .unwrap_or(0);
        lines.remove(weakest);
        dropped += 1;
    }
}
