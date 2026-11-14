mod common;

use common::*;
use reflex_core::events::{
    estimate_tokens, Event, EventClass, Upward, DEFAULT_DIGEST_TOKENS, RECOVERY_TICKS,
};
use reflex_core::snapshot::BlockKind;
use reflex_core::{BlockPos, Executor};

fn digest_ev(t: u64, kind: &str, subject: &str) -> Event {
    Event::new(t, EventClass::Digest, kind, subject)
}

#[test]
fn digest_stays_under_token_cap() {
    let mut up = Upward::default();
    for i in 0..200 {
        up.push(digest_ev(1000 + i, &format!("reflex_{i}"), "zombie").hp(20.0 - (i % 10) as f32));
    }
    let d = up.take_digest(DEFAULT_DIGEST_TOKENS);
    assert!(d.tokens <= DEFAULT_DIGEST_TOKENS, "{} tokens", d.tokens);
    assert_eq!(d.tokens, estimate_tokens(&d.text));
    assert!(d.dropped > 0 && d.text.contains("more"));
    assert!(
        up.take_digest(DEFAULT_DIGEST_TOKENS).text.is_empty(),
        "drained"
    );
}

#[test]
fn digest_aggregates_counts_and_hp() {
    let mut up = Upward::default();
    up.push(digest_ev(100, "lava_withdraw", "").hp(18.0));
    up.push(digest_ev(4000, "lava_withdraw", "").hp(14.0));
    let d = up.take_digest(150);
    assert_eq!(d.text, "t+0 lava_withdraw x2 hp 18->14");
}

#[test]
fn repeated_events_fade_and_recover() {
    let mut up = Upward::default();
    for t in 0..6 {
        up.push(digest_ev(1000 + t, "creeper_back_away", "").magnitude(1.0));
    }
    let d = up.take_digest(150);
    assert!(
        d.text.ends_with("x3"),
        "1, 0.5 and 0.25 pass; 0.125 is habituated: {}",
        d.text
    );

    up.push(digest_ev(1010, "creeper_back_away", "").magnitude(6.0));
    assert!(
        up.take_digest(150).text.contains("x1"),
        "a large change dishabituates"
    );

    let later = 1010 + (RECOVERY_TICKS * 4.0) as u64;
    up.push(digest_ev(later, "creeper_back_away", "").magnitude(6.0));
    assert!(
        up.take_digest(150).text.contains("x1"),
        "recovered after time"
    );
}

#[test]
fn ignore_and_attend_seed_and_break_habituation() {
    let mut up = Upward::default();
    up.habituation.ignore("orient_looming", 5000);
    up.push(digest_ev(1000, "orient_looming", "chicken"));
    assert!(up.take_digest(150).text.is_empty());
    up.habituation.attend("orient_looming");
    up.push(digest_ev(1001, "orient_looming", "chicken"));
    assert!(!up.take_digest(150).text.is_empty());
}

#[test]
fn wake_rate_limit_and_critical_bypass() {
    let mut up = Upward::default();
    up.push(Event::new(1000, EventClass::Wake, "chat", "human").text("a"));
    assert_eq!(up.take_wake(1000).len(), 1);
    up.push(Event::new(1010, EventClass::Wake, "chat", "human").text("b"));
    assert!(up.take_wake(1010).is_empty(), "within 5 s");
    assert_eq!(up.take_wake(1100).len(), 1, "after 100 ticks");
    up.push(
        Event::new(1101, EventClass::Wake, "override", "R5")
            .text("x")
            .critical(),
    );
    assert_eq!(up.take_wake(1101).len(), 1, "critical bypasses the limit");
}

#[test]
fn surprise_filter_drops_self_caused_and_already_told() {
    let mut up = Upward::default();
    up.push(digest_ev(1000, "block_changed", "").self_caused());
    assert!(
        up.take_digest(150).text.is_empty(),
        "efference copy: self-caused"
    );
    up.push(Event::new(1000, EventClass::Wake, "companion_far", "human").text("40 blocks"));
    up.push(Event::new(1200, EventClass::Wake, "companion_far", "human").text("40 blocks"));
    assert_eq!(
        up.take_wake(1300).len(),
        1,
        "second report repeats what the brain was told"
    );
    assert_eq!(up.log().count(), 3, "everything is logged");
}

#[test]
fn executor_reports_reflex_onset_once_into_the_digest() {
    let mut s = flat();
    s.blocks.set(BlockPos::new(0, 64, 0), BlockKind::Lava);
    let mut ex = Executor::new(BOT);
    for t in 0..5 {
        s.tick = 1000 + t;
        ex.tick(&s);
    }
    let d = ex.take_digest();
    assert_eq!(d.text.matches("lava_withdraw").count(), 1, "{}", d.text);
    assert!(d.text.contains("x1"));
}
