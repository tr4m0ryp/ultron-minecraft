//! Tool results as terse text: every token the brain reads costs.

use serde_json::Value;

use crate::wire::{ChatLine, Events, LogPage, Observation};

pub fn render(tool: &str, data: &Value) -> String {
    let parsed = match tool {
        "observe" => serde_json::from_value(data.clone())
            .ok()
            .map(|o| observation(&o)),
        "wait_event" => serde_json::from_value(data.clone())
            .ok()
            .map(|e| events(&e)),
        "recent_chat" => serde_json::from_value::<Vec<ChatLine>>(data.clone())
            .ok()
            .map(|c| chat(&c)),
        "log" => serde_json::from_value(data.clone()).ok().map(|p| log(&p)),
        _ => data
            .get("done")
            .and_then(Value::as_str)
            .map(|d| format!("ok: {d}")),
    };
    parsed.unwrap_or_else(|| data.to_string())
}

fn observation(o: &Observation) -> String {
    let [x, y, z] = o.pos;
    let mut s = format!(
        "t={} pos {x},{y},{z} hp {} food {} held {} {} mode {}",
        o.tick,
        o.hp,
        o.food,
        o.held.as_deref().unwrap_or("nothing"),
        if o.night { "night" } else { "day" },
        o.mode
    );
    let near: Vec<String> = o
        .near
        .iter()
        .map(|g| match g.n {
            1 => format!("{} {}", g.kind, g.dist),
            n => format!("{} x{n} {}", g.kind, g.dist),
        })
        .collect();
    s.push_str(&format!(
        "\nnear: {}",
        if near.is_empty() {
            "nothing".into()
        } else {
            near.join(", ")
        }
    ));
    match &o.companion {
        Some(c) => {
            s.push_str(&format!("\ncompanion {} {}m", c.name, c.dist));
            if let Some(hp) = c.hp {
                s.push_str(&format!(" hp {hp}"));
            }
        }
        None => s.push_str("\ncompanion not visible"),
    }
    match &o.goal {
        Some(g) => {
            s.push_str(&format!(
                "\ngoal #{} {} p{} {}",
                g.id, g.spec, g.priority, g.status
            ));
            if let Some(p) = &g.progress {
                s.push_str(&format!(" {p}"));
            }
            if let Some(n) = &g.note {
                s.push_str(&format!(" ({n})"));
            }
            s.push_str(&format!(", {} queued", g.queued));
        }
        None => s.push_str("\ngoal none"),
    }
    if let Some(last) = &o.last_goal {
        s.push_str(&format!("\nlast goal {last}"));
    }
    if !o.digest.is_empty() {
        s.push_str(&format!("\ndigest: {}", o.digest));
    }
    if o.wakes_pending > 0 {
        s.push_str(&format!(
            "\n{} events pending: call wait_event",
            o.wakes_pending
        ));
    }
    s
}

fn events(e: &Events) -> String {
    if e.events.is_empty() && e.idle {
        return "no event: the body has no goal and is standing idle. Take the next step of \
                your project now (at night too: craft, build or fix at the base)"
            .into();
    }
    if e.events.is_empty() {
        return "no event (timeout)".into();
    }
    e.events
        .iter()
        .map(|w| {
            let mut s = format!("t={} {}", w.t, w.kind);
            if !w.subject.is_empty() && !w.text.contains(&w.subject) {
                s.push_str(&format!(" {}", w.subject));
            }
            if !w.text.is_empty() {
                s.push_str(&format!(": {}", w.text));
            }
            s
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn chat(lines: &[ChatLine]) -> String {
    if lines.is_empty() {
        return "no chat yet".into();
    }
    lines
        .iter()
        .map(|c| format!("t={} <{}> {}", c.t, c.from, c.text))
        .collect::<Vec<_>>()
        .join("\n")
}

fn log(p: &LogPage) -> String {
    let mut s = format!("page {} ({} events total, newest first)", p.page, p.total);
    for l in &p.lines {
        s.push_str(&format!(
            "\nt={} {} {} {} {}",
            l.t, l.class, l.kind, l.subject, l.text
        ));
    }
    s
}
