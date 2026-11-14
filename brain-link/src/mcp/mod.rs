//! MCP tool server over stdio (hand-rolled JSON-RPC 2.0) that forwards each tool call to
//! the executor socket and renders the reply as compact text.

mod call;
mod render;
mod tools;

pub use tools::{tool_list, TOOL_NAMES};

use std::io::{BufRead, Write};
use std::path::PathBuf;

use serde_json::{json, Value};

const PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
const INSTRUCTIONS: &str = "You are a player in a Minecraft world. A fast local executor keeps the body alive and carries out your goals; you decide what to do. Loop: wait_event, think, act (set_goal, set_mode, set_gain, inhibit, arm, say), then wait_event again.";

/// Serves JSON-RPC requests from `input` until it closes.
pub fn serve(input: impl BufRead, mut output: impl Write, socket: PathBuf) -> std::io::Result<()> {
    let mut link = call::Link::new(socket);
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = handle(&line, &mut link) {
            writeln!(output, "{reply}")?;
            output.flush()?;
        }
    }
    Ok(())
}

fn handle(line: &str, link: &mut call::Link) -> Option<Value> {
    let msg: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return Some(error(Value::Null, -32700, &format!("parse error: {e}"))),
    };
    let id = msg.get("id").cloned()?;
    let params = msg.get("params").cloned().unwrap_or(json!({}));
    let result = match msg.get("method").and_then(Value::as_str) {
        Some("initialize") => {
            let asked = params.get("protocolVersion").and_then(Value::as_str);
            let version = asked
                .filter(|v| PROTOCOL_VERSIONS.contains(v))
                .unwrap_or(PROTOCOL_VERSIONS[0]);
            json!({
                "protocolVersion": version,
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {"name": "minecraft", "version": env!("CARGO_PKG_VERSION")},
                "instructions": INSTRUCTIONS,
            })
        }
        Some("ping") => json!({}),
        Some("tools/list") => json!({"tools": tool_list()}),
        Some("tools/call") => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            if !TOOL_NAMES.contains(&name) {
                return Some(error(id, -32602, &format!("unknown tool: {name}")));
            }
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            let (text, is_error) = match link.call(name, args) {
                Ok(data) => (render::render(name, &data), false),
                Err(e) => (format!("error: {e}"), true),
            };
            json!({"content": [{"type": "text", "text": text}], "isError": is_error})
        }
        Some(m) => return Some(error(id, -32601, &format!("method not found: {m}"))),
        None => return Some(error(id, -32600, "invalid request: no method")),
    };
    Some(json!({"jsonrpc": "2.0", "id": id, "result": result}))
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}
