//! Upward channel to stdout: WAKE events as they become due, the digest every few seconds.

use reflex_core::Executor;

/// Ticks between digest flushes (10 s).
const DIGEST_EVERY: u64 = 200;

pub fn publish(exec: &mut Executor, now: u64) {
    for e in exec.take_wake(now) {
        println!("WAKE t={} {} {} {}", e.tick, e.kind, e.subject, e.text);
    }
    if now.is_multiple_of(DIGEST_EVERY) {
        let d = exec.take_digest();
        if !d.text.is_empty() {
            println!(
                "DIGEST t={now} dropped={}\n{}",
                d.dropped,
                d.text.trim_end()
            );
        }
    }
}

/// Flight-recorder dump after a goal stall or failure: `TRACE` lines, run-length encoded, so
/// the play log stays readable and `grep TRACE` pulls out the lead-up to any stall.
pub fn dump_trace(exec: &mut Executor) {
    for line in exec.take_trace_dump() {
        println!("{line}");
    }
}
