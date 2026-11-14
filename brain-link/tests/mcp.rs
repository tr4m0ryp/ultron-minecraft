//! Drives the MCP binary over stdio against the fake executor.

mod common;

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use common::Rig;
use serde_json::{json, Value};

struct Mcp {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next: u64,
}

impl Mcp {
    fn spawn(sock: &Path) -> Mcp {
        let mut child = Command::new(env!("CARGO_BIN_EXE_brain-link-mcp"))
            .env("ULTRON_BRAIN_SOCK", sock)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("spawn mcp");
        Mcp {
            stdin: child.stdin.take().unwrap(),
            stdout: BufReader::new(child.stdout.take().unwrap()),
            child,
            next: 0,
        }
    }

    fn line(&mut self, raw: &str) -> Value {
        writeln!(self.stdin, "{raw}").unwrap();
        let mut s = String::new();
        self.stdout.read_line(&mut s).unwrap();
        serde_json::from_str(&s).unwrap()
    }

    fn rpc(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let msg = json!({"jsonrpc":"2.0","id":self.next,"method":method,"params":params});
        let reply = self.line(&msg.to_string());
        assert_eq!(reply["id"], self.next);
        reply
    }

    /// Returns (text, is_error) of a tool call.
    fn tool(&mut self, name: &str, args: Value) -> (String, bool) {
        let r = self.rpc("tools/call", json!({"name": name, "arguments": args}));
        let res = &r["result"];
        (
            res["content"][0]["text"].as_str().unwrap().to_string(),
            res["isError"].as_bool().unwrap(),
        )
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn handshake_and_tool_list() {
    let rig = Rig::start();
    let mut m = Mcp::spawn(&rig.path);
    let init = m.rpc("initialize", json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}));
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(init["result"]["serverInfo"]["name"], "minecraft");
    writeln!(
        m.stdin,
        r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#
    )
    .unwrap();
    assert_eq!(m.rpc("ping", json!({}))["result"], json!({}));
    let list = m.rpc("tools/list", json!({}));
    let tools = list["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, brain_link::mcp::TOOL_NAMES);
    for t in tools {
        assert_eq!(t["inputSchema"]["type"], "object");
        assert_eq!(t["inputSchema"]["additionalProperties"], false);
        let d = t["description"].as_str().unwrap();
        assert!(!d.contains('\u{2014}') && d.len() < 700, "{d}");
    }
    let unknown = m.rpc("tools/call", json!({"name":"fly","arguments":{}}));
    assert_eq!(unknown["error"]["code"], -32602);
    assert_eq!(m.rpc("resources/list", json!({}))["error"]["code"], -32601);
    assert_eq!(m.line("{oops")["error"]["code"], -32700);
}

#[test]
fn tools_round_trip_compactly() {
    let rig = Rig::start();
    let mut m = Mcp::spawn(&rig.path);
    let (obs, err) = m.tool("observe", json!({}));
    assert!(!err, "{obs}");
    assert!(
        obs.contains("held sword") && obs.contains("companion Steve 3m"),
        "{obs}"
    );
    assert!(obs.len() < 400, "observe too long: {}", obs.len());

    let (t, err) = m.tool(
        "set_goal",
        json!({"goal":"go_to","params":{"pos":[9000,64,0]}}),
    );
    assert!(!err && t.starts_with("ok: goal #1 go_to 9000 64 0"), "{t}");
    let (obs, _) = m.tool("observe", json!({}));
    assert!(
        obs.contains("goal #1 go_to 9000 64 0 p0.5 running"),
        "{obs}"
    );

    for (name, args) in [
        ("set_mode", json!({"mode":"fight"})),
        ("set_gain", json!({"gain":"caution","value":0.7})),
        (
            "inhibit",
            json!({"reflex":"R5","scope":{"kind":"until_goal_ends"}}),
        ),
        (
            "arm",
            json!({"trigger":{"kind":"hp_below","hp":6},"response":{"kind":"say","text":"help"}}),
        ),
        ("say", json!({"text":"hello"})),
    ] {
        let (t, err) = m.tool(name, args);
        assert!(!err && t.starts_with("ok: "), "{name}: {t}");
    }
    let caution = rig.with(|w| w.exec.knobs.gains.caution);
    assert_eq!(caution, 0.7);

    let (t, _) = m.tool("wait_event", json!({"timeout_s":0.2}));
    assert_eq!(
        t, "no event (timeout)",
        "a goal runs, so the body is not idle"
    );
    let (t, _) = m.tool("set_goal", json!({"goal":"end"}));
    assert!(t.starts_with("ok: ended #1"), "{t}");
    // The goal ended, so the next wait returns at once and reports the body idle.
    let t0 = std::time::Instant::now();
    let (t, _) = m.tool("wait_event", json!({"timeout_s":30}));
    assert!(
        t.starts_with("no event (timeout)") && t.contains("standing idle"),
        "{t}"
    );
    assert!(
        t0.elapsed() < std::time::Duration::from_secs(2),
        "nudge was not immediate"
    );
    rig.wake("chat", "Steve", "<Steve> come here");
    let (t, _) = m.tool("wait_event", json!({"timeout_s":5}));
    assert!(
        t.starts_with("t=") && t.ends_with(" chat: <Steve> come here"),
        "{t}"
    );

    let (t, _) = m.tool("recent_chat", json!({"limit":3}));
    assert!(t.contains("<Ultron> hello"), "{t}");
    let (t, _) = m.tool("log", json!({"page_size":2}));
    assert!(
        t.starts_with("page 0 (1 events") && t.contains("W chat Steve <Steve> come here"),
        "{t}"
    );

    let (t, err) = m.tool(
        "set_goal",
        json!({"goal":"place","params":{"kind":"stone"}}),
    );
    assert!(err && t.contains("needs param 'pos'"), "{t}");
    let (t, err) = m.tool("set_gain", json!({"gain":"caution"}));
    assert!(err && t.contains("missing field `value`"), "{t}");
}

#[test]
fn reports_a_missing_executor_and_recovers_when_it_appears() {
    let path = common::temp_socket();
    let mut m = Mcp::spawn(&path);
    let (t, err) = m.tool("observe", json!({}));
    assert!(err && t.contains("not reachable"), "{t}");
    let (_bridge, _server) = brain_link::Bridge::mount(&path).unwrap();
    let (t, err) = m.tool("wait_event", json!({"timeout_s":0.1}));
    assert!(!err && t == "no event (timeout)", "{t}");
}
