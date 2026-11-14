use std::path::PathBuf;

use serde_json::Value;

use crate::client::Client;
use crate::wire;

/// A lazily (re)connected socket client: the tool server starts before the game does.
pub struct Link {
    path: PathBuf,
    client: Option<Client>,
}

impl Link {
    pub fn new(path: PathBuf) -> Self {
        Self { path, client: None }
    }

    /// Forwards one tool call; the tool name is the request's `op`.
    pub fn call(&mut self, tool: &str, args: Value) -> Result<Value, String> {
        let mut req = match args {
            Value::Object(m) => m,
            Value::Null => Default::default(),
            _ => return Err("arguments must be an object".into()),
        };
        req.insert("op".into(), Value::String(tool.into()));
        let req = Value::Object(req);
        let reused = self.client.is_some();
        let reply = match self.send(&req) {
            Err(_) if reused => self.send(&req),
            other => other,
        };
        wire::open(reply?)
    }

    /// Returns the raw reply envelope; `Err` only for transport failures.
    fn send(&mut self, req: &Value) -> Result<Value, String> {
        if self.client.is_none() {
            let c = Client::connect(&self.path).map_err(|e| {
                format!(
                    "executor not reachable at {} ({e}); is the bot running?",
                    self.path.display()
                )
            })?;
            self.client = Some(c);
        }
        let client = self.client.as_mut().expect("connected above");
        match client.raw(req) {
            Ok(reply) => Ok(reply),
            Err(e) => {
                self.client = None;
                Err(e)
            }
        }
    }
}
