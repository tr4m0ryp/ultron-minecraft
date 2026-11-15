use std::fmt::Write;

use super::goal::{FLAG_JUMP, FLAG_SNEAK, FLAG_SPRINT};
use super::ring::{Decision, TickRec, TickRing};

/// Runs printed at most; older ones are summarised so a long walk cannot flood the log.
pub const MAX_RUNS: usize = 60;

const CHANNELS: [&str; 5] = ["legs", "gaze", "main", "off", "inv"];

/// One run of consecutive ticks with equal decisions.
struct Run {
    first: TickRec,
    last: TickRec,
    n: u32,
}

/// The decision without the per-hop detail (next waypoint, jump and sprint), so a walk along
/// a path is one run; the run prints the waypoint it ended on.
fn same_run(a: &Decision, b: &Decision) -> bool {
    let norm = |d: &Decision| {
        let mut d = *d;
        d.goal.wp = [0; 3];
        d.goal.flags &= FLAG_SNEAK;
        d
    };
    norm(a) == norm(b)
}

fn runs(ring: &TickRing) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    for rec in ring.iter() {
        match out.last_mut() {
            Some(r) if same_run(&r.last.d, &rec.d) && rec.tick == r.last.tick + 1 => {
                r.last = *rec;
                r.n += 1;
            }
            _ => out.push(Run {
                first: *rec,
                last: *rec,
                n: 1,
            }),
        }
    }
    out
}

fn decision(d: &Decision, s: &mut String) {
    for (name, id) in CHANNELS.iter().zip(d.owners) {
        if !id.is_empty() {
            let _ = write!(s, " {name}={id}");
        }
    }
    if !d.legs_note.is_empty() {
        let _ = write!(s, "/{}", d.legs_note);
    }
    if !d.vetoes[0].is_empty() {
        s.push_str(" veto=");
        let mut first = true;
        for v in d.vetoes.iter().filter(|v| !v.is_empty()) {
            if !first {
                s.push(',');
            }
            first = false;
            s.push_str(v);
        }
    }
    let g = &d.goal;
    if g.goal_id != 0 {
        let _ = write!(s, " goal=#{} {}", g.goal_id, g.label);
        if !g.phase.is_empty() {
            let _ = write!(s, "/{}", g.phase);
        }
        let _ = write!(s, " nav={}", g.nav.label());
        if g.wp != [0; 3] {
            let _ = write!(s, " wp={},{},{}", g.wp[0], g.wp[1], g.wp[2]);
        }
        for (bit, name) in [
            (FLAG_JUMP, "jump"),
            (FLAG_SPRINT, "sprint"),
            (FLAG_SNEAK, "sneak"),
        ] {
            if g.flags & bit != 0 {
                let _ = write!(s, " {name}");
            }
        }
        if g.preempted {
            s.push_str(" PREEMPTED");
        }
        if g.blocked {
            s.push_str(" blocked");
        }
    }
}

fn pos(p: [f32; 3]) -> String {
    format!("{:.1},{:.1},{:.1}", p[0], p[1], p[2])
}

/// Header plus one line per run, oldest first. Lines start with `TRACE` so the play log can
/// be filtered; a still body shows as one long run whose first and last position match.
pub fn render(ring: &TickRing, reason: &str) -> Vec<String> {
    let runs = runs(ring);
    let total = runs.len();
    let skip = total.saturating_sub(MAX_RUNS);
    let mut lines = Vec::with_capacity(total - skip + 1);
    lines.push(format!(
        "TRACE dump: {reason} ({} ticks, {total} runs{})",
        ring.len(),
        if skip > 0 {
            format!(", oldest {skip} omitted")
        } else {
            String::new()
        }
    ));
    for r in runs.into_iter().skip(skip) {
        let mut s = if r.n == 1 {
            format!("TRACE t={}", r.first.tick)
        } else {
            format!("TRACE t={}..{} x{}", r.first.tick, r.last.tick, r.n)
        };
        if r.first.pos == r.last.pos {
            let _ = write!(s, " pos={}", pos(r.first.pos));
        } else {
            let _ = write!(s, " pos={}->{}", pos(r.first.pos), pos(r.last.pos));
        }
        decision(&r.last.d, &mut s);
        lines.push(s);
    }
    lines
}
