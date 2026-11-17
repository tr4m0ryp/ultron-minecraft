//! Block probes for staged measurements, read from stdin beside the brain's commands:
//! `probe <x> <y> <z>` prints the block the client's world holds there once, and
//! `watch <x> <y> <z>` prints it now and again on every tick it changes (`unwatch` drops
//! all watches). The lines carry the body's tick, so a staged script can tell on which
//! tick a change on the server became visible to the body.
//!
//! `B <tick> <x> <y> <z> <block>`; a cell the client has no chunk for prints `unloaded`.

use reflex_core::math::BlockPos;

/// Most cells watched at once; a watch is a few hash lookups per tick.
const MAX_WATCHED: usize = 64;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ProbeCmd {
    Probe(BlockPos),
    Watch(BlockPos),
    Unwatch,
}

/// Parses a probe line; `None` when the line is something else.
pub fn parse(line: &str) -> Option<ProbeCmd> {
    let mut it = line.split_whitespace();
    let head = it.next()?;
    if head == "unwatch" {
        return Some(ProbeCmd::Unwatch);
    }
    let mut num = || it.next()?.parse::<i32>().ok();
    let pos = BlockPos::new(num()?, num()?, num()?);
    match head {
        "probe" => Some(ProbeCmd::Probe(pos)),
        "watch" => Some(ProbeCmd::Watch(pos)),
        _ => None,
    }
}

#[derive(Default)]
pub struct Watches {
    cells: Vec<(BlockPos, Option<String>)>,
}

impl Watches {
    pub fn apply(&mut self, cmd: ProbeCmd, tick: u64, read: impl Fn(BlockPos) -> Option<String>) {
        match cmd {
            ProbeCmd::Probe(p) => print_cell(tick, p, read(p).as_deref()),
            ProbeCmd::Watch(p) => {
                if self.cells.len() < MAX_WATCHED && !self.cells.iter().any(|(c, _)| *c == p) {
                    let now = read(p);
                    print_cell(tick, p, now.as_deref());
                    self.cells.push((p, now));
                }
            }
            ProbeCmd::Unwatch => self.cells.clear(),
        }
    }

    /// Prints every watched cell whose block changed since the last tick.
    pub fn tick(&mut self, tick: u64, read: impl Fn(BlockPos) -> Option<String>) {
        for (p, last) in &mut self.cells {
            let now = read(*p);
            if now != *last {
                print_cell(tick, *p, now.as_deref());
                *last = now;
            }
        }
    }
}

fn print_cell(tick: u64, p: BlockPos, name: Option<&str>) {
    println!(
        "B {tick} {} {} {} {}",
        p.x,
        p.y,
        p.z,
        name.unwrap_or("unloaded")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_lines_parse_and_other_lines_do_not() {
        let p = BlockPos::new(1, -2, 3);
        assert_eq!(parse("probe 1 -2 3"), Some(ProbeCmd::Probe(p)));
        assert_eq!(parse("watch 1 -2 3"), Some(ProbeCmd::Watch(p)));
        assert_eq!(parse("unwatch"), Some(ProbeCmd::Unwatch));
        assert_eq!(parse("watch 1 2"), None);
        assert_eq!(parse("task mine stone 3"), None);
    }

    #[test]
    fn a_watch_keeps_the_last_block_it_saw() {
        let p = BlockPos::new(0, 64, 0);
        let mut w = Watches::default();
        w.apply(ProbeCmd::Watch(p), 1, |_| Some("stone".into()));
        assert_eq!(w.cells[0].1.as_deref(), Some("stone"));
        w.tick(2, |_| Some("air".into()));
        assert_eq!(w.cells[0].1.as_deref(), Some("air"));
        w.apply(ProbeCmd::Unwatch, 3, |_| None);
        assert!(w.cells.is_empty());
    }
}
