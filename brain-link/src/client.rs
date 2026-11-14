//! Blocking client for the executor socket.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use serde_json::Value;

use crate::bridge::{TICK_REPLY_TIMEOUT, WAIT_DEFAULT_S, WAIT_MAX_S};
use crate::wire::{self, Request};

pub struct Client {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl Client {
    pub fn connect(path: impl AsRef<Path>) -> std::io::Result<Client> {
        let stream = UnixStream::connect(path)?;
        Ok(Client {
            writer: stream.try_clone()?,
            reader: BufReader::new(stream),
        })
    }

    /// Sends one raw request object and returns the raw reply envelope.
    pub fn raw(&mut self, req: &Value) -> Result<Value, String> {
        let wait = match req.get("op").and_then(Value::as_str) {
            Some("wait_event") => req
                .get("timeout_s")
                .and_then(Value::as_f64)
                .unwrap_or(WAIT_DEFAULT_S),
            _ => 0.0,
        };
        let wait = if wait.is_finite() {
            wait.clamp(0.0, WAIT_MAX_S)
        } else {
            0.0
        };
        let read_timeout =
            TICK_REPLY_TIMEOUT + Duration::from_secs(2) + Duration::from_secs_f64(wait);
        self.writer
            .set_read_timeout(Some(read_timeout))
            .map_err(|e| e.to_string())?;
        let mut line = req.to_string();
        line.push('\n');
        self.writer
            .write_all(line.as_bytes())
            .map_err(|e| format!("send failed: {e}"))?;
        let mut reply = String::new();
        match self.reader.read_line(&mut reply) {
            Ok(0) => Err("executor closed the connection".into()),
            Ok(_) => serde_json::from_str(&reply).map_err(|e| format!("bad reply: {e}")),
            Err(e) => Err(format!("no reply: {e}")),
        }
    }

    /// Sends a typed request and returns the reply data or the error message.
    pub fn call(&mut self, req: &Request) -> Result<Value, String> {
        let v = serde_json::to_value(req).map_err(|e| e.to_string())?;
        wire::open(self.raw(&v)?)
    }
}
