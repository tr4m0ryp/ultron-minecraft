"""Drives the real tool path against the live bot: brain-link-mcp over stdio (JSON-RPC, one
message per line), the bot's socket behind it. Measures each call's latency and the size
of `observe`. Usage: python3 tools/live/staged/mcp_drive.py"""
import json, subprocess, sys, time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "goals"))
from kit import *  # noqa: E402

MCP = str(REPO / "target/release/brain-link-mcp")
SERVER_OUT = Path.home() / "minecraft/vanilla-26.3/server.out"


class Mcp:
    def __init__(self):
        self.p = subprocess.Popen([MCP], cwd=REPO, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  text=True, bufsize=1)
        self.n = 0
        self.timings = []

    def rpc(self, method, params=None):
        self.n += 1
        t = time.perf_counter()
        self.p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": self.n, "method": method,
                                       "params": params or {}}) + "\n")
        self.p.stdin.flush()
        reply = json.loads(self.p.stdout.readline())
        return reply, round((time.perf_counter() - t) * 1000, 1)

    def tool(self, name, args=None):
        reply, ms = self.rpc("tools/call", {"name": name, "arguments": args or {}})
        r = reply.get("result", {})
        text = "".join(c.get("text", "") for c in r.get("content", []))
        self.timings.append((name, ms))
        return text, bool(r.get("isError")), ms


def chat_seen(text, since):
    return any(text in l for l in SERVER_OUT.read_text(errors="replace").splitlines()[since:])


def main():
    build_site()
    start()
    out = {}
    try:
        prep(comp_at=(CX + 4.5, Y + 1, CZ + 0.5))
        m = Mcp()
        init, ms = m.rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                                        "clientInfo": {"name": "drive", "version": "0"}})
        m.p.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n")
        tools, ms_list = m.rpc("tools/list")
        out["init_ms"], out["list_ms"] = ms, ms_list
        out["tools"] = [t["name"] for t in tools["result"]["tools"]]
        obs, err, ms = m.tool("observe")
        out["observe"] = dict(chars=len(obs), ms=ms, err=err, text=obs)
        t, err, ms = m.tool("set_goal", {"goal": "follow", "params": {"distance": 3}})
        out["set_goal_follow"] = dict(text=t, err=err, ms=ms)
        rc(f"tp {COMP} {CX + 14.5} {Y + 1} {CZ + 6.5}")
        time.sleep(6)
        out["follow_dist_after_6s"] = dist(pos(), pos(COMP))
        obs, err, ms = m.tool("observe")
        out["observe_following"] = dict(chars=len(obs), ms=ms, text=obs)
        n0 = len(SERVER_OUT.read_text(errors="replace").splitlines())
        t, err, ms = m.tool("say", {"text": "hello from the tool path"})
        time.sleep(1.5)
        out["say"] = dict(text=t, ms=ms, seen_on_server=chat_seen("<Ultron> hello from the tool path", n0))
        m.tool("wait_event", {"timeout_s": 0.1})  # drain anything pending
        send("say are you there ultron", who="comp")
        t, err, ms = m.tool("wait_event", {"timeout_s": 10})
        out["wait_event_chat"] = dict(text=t, ms=ms, got=("are you there ultron" in t))
        t, err, ms = m.tool("arm", {"trigger": {"kind": "chat_matches", "pattern": "ping"},
                                    "response": {"kind": "say", "text": "pong"}})
        out["arm"] = dict(text=t, err=err, ms=ms)
        n0 = len(SERVER_OUT.read_text(errors="replace").splitlines())
        send("say ping", who="comp")
        time.sleep(2)
        out["armed_fired"] = chat_seen("<Ultron> pong", n0)
        t, err, ms = m.tool("recent_chat", {"limit": 4})
        out["recent_chat"] = dict(text=t, ms=ms)
        m.tool("set_goal", {"goal": "end"})
        rc(f"tp Ultron {CX + 0.5} {Y + 1} {CZ + 0.5}", f"tp {COMP} {CX + 14.5} {Y + 1} {CZ + 0.5}")
        t, err, ms = m.tool("set_gain", {"gain": "leash", "value": 6})
        t, err, ms = m.tool("inhibit", {"reflex": "H8", "ttl_s": 5})
        out["inhibit"] = dict(text=t, err=err, ms=ms)
        p0 = pos()
        time.sleep(3.5)
        held = dist(p0, pos())
        time.sleep(6)
        out["inhibit_effect"] = dict(moved_while_inhibited=held, moved_after_expiry=dist(p0, pos()),
                                     dist_to_companion_after=dist(pos(), pos(COMP)))
        t, err, ms = m.tool("wait_event", {"timeout_s": 2})
        out["wait_event_timeout"] = dict(text=t[:120], ms=ms)
        t, err, ms = m.tool("log", {"page_size": 5})
        out["log_ms"] = ms
        out["latency_ms"] = m.timings
        m.p.stdin.close()
        m.p.wait(timeout=5)
    finally:
        stop()
    print(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
