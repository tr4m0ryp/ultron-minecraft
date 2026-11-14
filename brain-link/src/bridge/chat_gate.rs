//! Holds back chat the player starts on its own. Replies to a player who spoke recently
//! always go through; an unprompted line (a new project, news, a need) goes through only
//! when the last one is at least `UNPROMPTED_GAP` old, and no line repeats a recent one.

use std::collections::VecDeque;

use reflex_core::snapshot::ChatEvent;

const TICKS_PER_S: u64 = 20;
/// After a player speaks, lines within this window count as replies.
pub const REPLY_WINDOW: u64 = 90 * TICKS_PER_S;
/// Minimum spacing between two unprompted lines.
pub const UNPROMPTED_GAP: u64 = 300 * TICKS_PER_S;
/// Own lines remembered for the repeat check and for telling echoes apart from players.
const SENT_KEEP: usize = 12;

#[derive(Default)]
pub struct ChatGate {
    last_heard: Option<u64>,
    last_unprompted: Option<u64>,
    sent: VecDeque<String>,
}

fn norm(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

impl ChatGate {
    /// Notes chat heard this tick. System lines (no sender) and echoes of own lines do not
    /// open a reply window.
    pub fn heard(&mut self, lines: &[ChatEvent], tick: u64) {
        let player = lines
            .iter()
            .any(|c| !c.sender.is_empty() && !self.sent.contains(&norm(&c.text)));
        if player {
            self.last_heard = Some(tick);
        }
    }

    /// Admits `line` at `tick` or says why it is held back.
    pub fn admit(&mut self, line: &str, tick: u64) -> Result<(), String> {
        let key = norm(line);
        if self.sent.contains(&key) {
            return Err(
                "not sent: you said that line recently; say something new or nothing".into(),
            );
        }
        let reply = self
            .last_heard
            .is_some_and(|t| tick.saturating_sub(t) <= REPLY_WINDOW);
        if !reply {
            if let Some(t) = self.last_unprompted {
                let ago = tick.saturating_sub(t);
                if ago < UNPROMPTED_GAP {
                    return Err(format!(
                        "not sent: nobody spoke to you and your last unprompted line was {} s \
                         ago; unprompted chat is limited to one line per {} s (a new project, \
                         real news or a need). Keep playing quietly; replies always go through",
                        ago / TICKS_PER_S,
                        UNPROMPTED_GAP / TICKS_PER_S
                    ));
                }
            }
            self.last_unprompted = Some(tick);
        }
        self.sent.push_back(key);
        while self.sent.len() > SENT_KEEP {
            self.sent.pop_front();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn said(sender: &str, text: &str) -> Vec<ChatEvent> {
        vec![ChatEvent {
            sender: sender.into(),
            text: text.into(),
            sender_pos: None,
        }]
    }

    #[test]
    fn unprompted_lines_are_spaced() {
        let mut g = ChatGate::default();
        assert!(g.admit("starting a farm", 100).is_ok());
        let e = g.admit("walls are up", 100 + 60 * TICKS_PER_S).unwrap_err();
        assert!(e.contains("one line per 300 s"), "{e}");
        assert!(g.admit("roof's on", 100 + UNPROMPTED_GAP).is_ok());
    }

    #[test]
    fn replies_always_pass_within_the_window() {
        let mut g = ChatGate::default();
        assert!(g.admit("starting a farm", 0).is_ok());
        g.heard(&said("tr4m0ryp", "hi"), 200);
        assert!(g.admit("hey!", 210).is_ok());
        assert!(g.admit("on my way", 220).is_ok());
        assert!(g.admit("got it", 200 + REPLY_WINDOW).is_ok());
        assert!(g.admit("anything else", 201 + REPLY_WINDOW).is_err());
    }

    #[test]
    fn repeats_and_echoes_are_caught() {
        let mut g = ChatGate::default();
        g.heard(&said("tr4m0ryp", "hi"), 0);
        assert!(g.admit("On my way!", 10).is_ok());
        assert!(g.admit("on my way", 20).is_err());
        let mut g = ChatGate::default();
        assert!(g.admit("starting a farm", 0).is_ok());
        g.heard(&said("Ultron", "starting a farm"), 1);
        g.heard(&said("", "Ultron joined the game"), 2);
        assert!(
            g.admit("next thing", 3).is_err(),
            "echo opened a reply window"
        );
    }
}
