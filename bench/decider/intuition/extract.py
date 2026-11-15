"""Recorded Thought decisions -> (state, decision) pairs.

  python extract.py [logs/brain] [bench/decider/data/pairs_real.jsonl]

Source: the stream-json transcripts the play relay writes (logs/brain/<run>/session-<n>.jsonl). One pair per
decision point: the session start (first observe) and every wait_event result. The decision is what Thought
did between that point and its next wait_event:
  goal name   if it set a goal (the last set_goal of the turn when there are several)
  say, arm, set_mode, inhibit   if it did only that (in this priority)
  wait        if it called wait_event again without acting
State is what the body could hand to a local model at that moment, no hindsight: the event text, the freshest
observe snapshot (Thought's own observe calls in that turn included), the previous decisions and the mode.
Journal reads and writes are not decisions. The output is derived from private logs and stays untracked.
"""
import glob, json, os, re, sys

P = "mcp__minecraft__"
PRIORITY = ("say", "arm", "set_mode", "inhibit")


def calls(path):
    """-> list of (tool name, input, result text) in order, minecraft tools only."""
    out, open_ = [], {}
    for line in open(path):
        try:
            d = json.loads(line)
        except ValueError:
            continue
        content = d.get("message", {}).get("content")
        if not isinstance(content, list):
            continue
        for b in content:
            if b.get("type") == "tool_use" and b["name"].startswith(P):
                open_[b["id"]] = len(out)
                out.append([b["name"][len(P):], b["input"], ""])
            elif b.get("type") == "tool_result" and b.get("tool_use_id") in open_:
                x = b.get("content")
                if isinstance(x, list):
                    x = " ".join(y.get("text", "") for y in x)
                out[open_[b["tool_use_id"]]][2] = x or ""
    return out


def clean(text):
    """Drop hook lines appended to tool results (wall clock, deadline) and collapse whitespace."""
    keep = [l for l in text.split("\n") if l.strip() and not l.startswith(("[clock]", "<system", "clock:"))]
    return " ; ".join(keep)


def goal_text(inp):
    p = inp.get("params") or {}
    bits = [inp.get("goal", "?")] + [str(p[k]) if not isinstance(p[k], list) else "pos" for k in sorted(p)]
    return " ".join(bits)


def pairs(path, session):
    cs, out = calls(path), []
    snap, mode, hist, event, turn, obs_in_turn = "", "", [], "session start", [], False

    def close():
        if event is None:
            return
        goals = [c for c in turn if c[0] == "set_goal"]
        if goals:
            label, detail = goals[-1][1].get("goal", "?"), goal_text(goals[-1][1])
        else:
            names = [c[0] for c in turn]
            label = next((n for n in PRIORITY if n in names), "wait")
            detail = label
        out.append({"session": session, "i": len(out), "event": event, "snapshot": snap, "mode": mode,
                    "history": hist[-3:], "label": label, "detail": detail, "observed": obs_in_turn,
                    "chat": bool(re.search(r"\bchat\b|<[^>]+> ", event))})
        hist.append(detail)

    for name, inp, res in cs:
        if name == "wait_event":
            close()
            event, turn, obs_in_turn = clean(res), [], False
        elif name == "observe":
            snap, obs_in_turn = clean(res), True
            m = re.search(r"\bmode (\w+)", snap)
            mode = m.group(1) if m else mode
        elif name in ("recent_chat", "log"):
            continue
        else:
            turn.append((name, inp))
            if name == "set_mode":
                mode = inp.get("mode", mode)
    return out          # the turn after the last wait_event is the hand-over, not a decision


def main():
    src = sys.argv[1] if len(sys.argv) > 1 else "logs/brain"
    dst = sys.argv[2] if len(sys.argv) > 2 else "bench/decider/data/pairs_real.jsonl"
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    n = 0
    with open(dst, "w") as f:
        for path in sorted(glob.glob(os.path.join(src, "*", "session-*.jsonl"))):
            session = os.path.basename(os.path.dirname(path)) + "/" + os.path.basename(path)[8:-6]
            for p in pairs(path, session):
                f.write(json.dumps(p) + "\n"); n += 1
    print(n, "pairs ->", dst)


if __name__ == "__main__":
    main()
