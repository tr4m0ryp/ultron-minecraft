use reflex_core::events::EventClass;
use serde_json::Value;

use crate::host::Host;
use crate::wire::{LogLine, LogPage};

const LOG_PAGE_DEFAULT: usize = 20;
const LOG_PAGE_MAX: usize = 100;

/// One page of the executor's event log, newest first.
pub fn page<H: Host>(host: &mut H, page: Option<usize>, size: Option<usize>) -> Value {
    let size = size.unwrap_or(LOG_PAGE_DEFAULT).clamp(1, LOG_PAGE_MAX);
    let page = page.unwrap_or(0);
    let log = host.upward();
    let total = log.log().count();
    let lines = log
        .log()
        .rev()
        .skip(page.saturating_mul(size))
        .take(size)
        .map(|e| LogLine {
            t: e.tick,
            class: match e.class {
                EventClass::Wake => 'W',
                EventClass::Digest => 'D',
                EventClass::Log => 'L',
            },
            kind: e.kind.clone(),
            subject: e.subject.clone(),
            text: e.text.clone(),
        })
        .collect();
    serde_json::to_value(LogPage { page, total, lines }).expect("log page serialises")
}
