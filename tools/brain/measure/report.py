"""Measures a brain relay run: reply latency, idle chat, tool use, tokens per minute.

usage: python3 report.py <run-dir> [--json]
Reads fake.log (stage lines `@<ms> <what>`), session-N.jsonl and session-N.ts.
"""
import glob
import json
import os
import re
import statistics
import sys

STAGE = re.compile(r"@(\d+) (chat in|chat out|goal set|goal_done|goal_failed): ?(.*)")
REPLY_WINDOW_MS = 90_000


def stage_events(run):
    out = []
    for line in open(os.path.join(run, "fake.log"), errors="replace"):
        m = STAGE.search(line)
        if m:
            out.append((int(m.group(1)), m.group(2), m.group(3)))
    return out


def session(run, n):
    base = os.path.join(run, f"session-{n}")
    lines = open(base + ".jsonl", errors="replace").read().splitlines()
    stamps = [int(s) for s in open(base + ".ts").read().split()]
    calls, results, final = [], {}, None
    for ts, raw in zip(stamps, lines):
        try:
            msg = json.loads(raw)
        except ValueError:
            continue
        kind = msg.get("type")
        content = (msg.get("message") or {}).get("content") or []
        if kind == "assistant":
            for c in content:
                if isinstance(c, dict) and c.get("type") == "tool_use":
                    calls.append({"ts": ts, "id": c["id"], "name": c["name"].split("__")[-1],
                                  "input": c.get("input", {})})
        elif kind == "user":
            for c in content if isinstance(content, list) else []:
                if isinstance(c, dict) and c.get("type") == "tool_result":
                    text = c.get("content")
                    if isinstance(text, list):
                        text = " ".join(t.get("text", "") for t in text if isinstance(t, dict))
                    results[c["tool_use_id"]] = (ts, bool(c.get("is_error")), str(text)[:200])
        elif kind == "result":
            final = msg
    for c in calls:
        c["result"] = results.get(c["id"])
    start = int(open(base + ".start").read())
    end = int(open(base + ".end").read()) if os.path.exists(base + ".end") else stamps[-1]
    return {"n": n, "start": start, "end": end, "calls": calls, "final": final}


def latency(events, sessions):
    calls = sorted((c for s in sessions for c in s["calls"]), key=lambda c: c["ts"])
    rows = []
    for t0, kind, text in events:
        if kind != "chat in":
            continue
        after = lambda k: next((t for t, kk, _ in events if kk == k and t0 < t <= t0 + REPLY_WINDOW_MS), None)
        say, goal = after("chat out"), after("goal set")
        woke = next((c["result"][0] for c in calls if c["name"] == "wait_event" and c["result"]
                     and c["ts"] < t0 <= c["result"][0]), None)
        trace = [f"{c['name']}@{(c['ts'] - t0) / 1000:.1f}" for c in calls
                 if t0 <= c["ts"] <= (say or t0 + REPLY_WINDOW_MS)]
        rows.append({"chat": text, "trace": trace, "say_s": say and (say - t0) / 1000,
                     "goal_s": goal and (goal - t0) / 1000,
                     "wake_s": woke and (woke - t0) / 1000})
    return rows


def unprompted(events):
    chats = [t for t, k, _ in events if k == "chat in"]
    return [(t, x) for t, k, x in events if k == "chat out"
            and not any(c < t <= c + REPLY_WINDOW_MS for c in chats)]


def usage(s):
    f = s["final"] or {}
    u = f.get("usage") or {}
    minutes = (s["end"] - s["start"]) / 60000
    total = sum(u.get(k, 0) for k in ("input_tokens", "output_tokens",
                                      "cache_creation_input_tokens", "cache_read_input_tokens"))
    return {"minutes": round(minutes, 2), "turns": f.get("num_turns"),
            "subtype": f.get("subtype"), "cost_usd": f.get("total_cost_usd"),
            "input": u.get("input_tokens"), "output": u.get("output_tokens"),
            "cache_write": u.get("cache_creation_input_tokens"),
            "cache_read": u.get("cache_read_input_tokens"),
            "tokens_per_min": round(total / minutes) if minutes else None,
            "uncached_per_min": round((total - u.get("cache_read_input_tokens", 0)) / minutes)
            if minutes else None}


def stats(xs):
    xs = [x for x in xs if x is not None]
    if not xs:
        return "no samples"
    return f"median {statistics.median(xs):.1f} s, worst {max(xs):.1f} s, n={len(xs)}"


def main():
    run = sys.argv[1]
    ns = sorted(int(re.search(r"session-(\d+)\.jsonl", p).group(1))
                for p in glob.glob(os.path.join(run, "session-*.jsonl")))
    sessions = [session(run, n) for n in ns]
    events = stage_events(run)
    lat = latency(events, sessions)
    report = {"latency": lat, "say": stats([r["say_s"] for r in lat]),
              "goal": stats([r["goal_s"] for r in lat]),
              "unprompted_chat": [x for _, x in unprompted(events)],
              "sessions": [dict(usage(s), n=s["n"], tools=[
                  (c["name"], c["input"], c["result"] and c["result"][1]) for c in s["calls"]])
                  for s in sessions]}
    if "--json" in sys.argv:
        print(json.dumps(report, indent=1, default=str))
        return
    print("reply latency, say:", report["say"])
    print("reply latency, set_goal:", report["goal"])
    for r in lat:
        print(f"  {r['say_s']}s say, {r['goal_s']}s goal, wake {r['wake_s']}s | {r['chat']}")
        print(f"     calls: {' '.join(r['trace'])}")
    print("unprompted chat lines:", len(report["unprompted_chat"]))
    for x in report["unprompted_chat"]:
        print("  ", x)
    for s in report["sessions"]:
        tools = s.pop("tools")
        print("session", s)
        for name, inp, err in tools:
            print(f"   {'ERR ' if err else ''}{name} {json.dumps(inp)[:150]}")


if __name__ == "__main__":
    main()
