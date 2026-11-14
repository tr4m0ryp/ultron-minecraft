"""Reads one relay session into an ordered list of tool calls and results.

Each session-N.jsonl line has its arrival time (unix ms) on the same line of session-N.ts.
"""
import glob
import json
import os
import re


def session_numbers(run):
    return sorted(int(re.search(r"session-(\d+)\.jsonl$", p).group(1))
                  for p in glob.glob(os.path.join(run, "session-*.jsonl")))


def _text(content):
    if isinstance(content, list):
        return " ".join(t.get("text", "") for t in content if isinstance(t, dict))
    return str(content or "")


def load(run, n):
    """Returns {n, start, end, steps}; a step is a dict with ts, kind (call|result|text),
    name (short tool name), input, text, error. Results carry the call's name."""
    base = os.path.join(run, f"session-{n}")
    lines = open(base + ".jsonl", errors="replace").read().splitlines()
    stamps = [int(s) for s in open(base + ".ts").read().split()]
    names, steps = {}, []
    for ts, raw in zip(stamps, lines):
        try:
            msg = json.loads(raw)
        except ValueError:
            continue
        content = (msg.get("message") or {}).get("content") or []
        if not isinstance(content, list):
            continue
        for c in content:
            if not isinstance(c, dict):
                continue
            if msg.get("type") == "assistant" and c.get("type") == "tool_use":
                name = c["name"].split("__")[-1]
                names[c["id"]] = name
                steps.append({"ts": ts, "kind": "call", "id": c["id"], "name": name,
                              "input": c.get("input") or {}})
            elif msg.get("type") == "assistant" and c.get("type") == "text":
                steps.append({"ts": ts, "kind": "text", "text": c.get("text", "")})
            elif msg.get("type") == "user" and c.get("type") == "tool_result":
                steps.append({"ts": ts, "kind": "result", "id": c["tool_use_id"],
                              "name": names.get(c["tool_use_id"], "?"),
                              "text": _text(c.get("content")), "error": bool(c.get("is_error"))})
    start = int(open(base + ".start").read())
    end = int(open(base + ".end").read()) if os.path.exists(base + ".end") else stamps[-1]
    return {"n": n, "start": start, "end": end, "steps": steps}
