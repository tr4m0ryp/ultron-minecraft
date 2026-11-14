use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;

use serde_json::Value;

use crate::bridge::Bridge;
use crate::wire::{self, Request};

/// Longest request line accepted; a longer one closes the connection.
pub const LINE_MAX: u64 = 64 * 1024;

pub fn serve(stream: UnixStream, bridge: &Bridge) {
    let Ok(mut out) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.by_ref().take(LINE_MAX).read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        if !line.ends_with('\n') && line.len() as u64 >= LINE_MAX {
            let _ = send(&mut out, &wire::err("request line too long"));
            return;
        }
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Request>(line.trim()) {
            Ok(req) => bridge.handle(req),
            Err(e) => wire::err(format!("bad request: {e}")),
        };
        if send(&mut out, &reply).is_err() {
            return;
        }
    }
}

fn send(out: &mut UnixStream, reply: &Value) -> std::io::Result<()> {
    let mut s = reply.to_string();
    s.push('\n');
    out.write_all(s.as_bytes())?;
    out.flush()
}
