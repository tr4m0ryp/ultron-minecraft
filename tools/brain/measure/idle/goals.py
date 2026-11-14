"""Rebuilds, from what the thought layer saw, when the body had a running goal and when the
thought layer sat blocked in wait_event.

Goal state follows the tool results in arrival order:
  set_goal `ok: goal #K` starts K and queues the goal it replaces (at most 3 queued);
  set_goal `ok: ended #K ...; active now: none|#J` ends K;
  a `goal_done|goal_failed|goal_interrupted: #K` event ends K at the wall time of its
    tick (mapped through observe readings, else its arrival); a queued goal then resumes;
  observe `goal none` and a wait_event idle timeout both mean nothing runs;
  observe `goal #K ... running|blocked|...` means K runs.
"""
import re

QUEUE_MAX = 3
MS_PER_TICK = 50
EVENT = re.compile(r"t=(\d+) (goal_done|goal_failed|goal_interrupted): #(\d+)")
OBS_TICK = re.compile(r"^t=(\d+) pos ")
OBS_GOAL = re.compile(r"\ngoal (none|#(\d+))")


def _tick_offsets(steps):
    """(arrival ms, ms offset) per observe: wall time of tick T is T * 50 + offset."""
    out = []
    for st in steps:
        if st["kind"] == "result" and st["name"] == "observe":
            m = OBS_TICK.match(st["text"])
            if m:
                out.append((st["ts"], st["ts"] - int(m.group(1)) * MS_PER_TICK))
    return out


def _event_wall(tick, arrived, offsets, floor):
    """Wall time of an event at `tick` that arrived at `arrived`; the arrival when no
    observe reading maps it plausibly (the bot may have restarted its tick count)."""
    best = None
    for _, off in offsets:
        wall = tick * MS_PER_TICK + off
        if floor <= wall <= arrived and (best is None or wall > best):
            best = wall
    return best if best is not None else arrived


def timeline(session):
    """Returns (goal_spans, wait_spans, idle_marks): spans are (start ms, end ms, label)
    clipped to the session; idle_marks are arrival times of idle wait_event timeouts."""
    steps, start, end = session["steps"], session["start"], session["end"]
    offsets = _tick_offsets(steps)
    running, since, queue, spans = None, None, [], []
    waits, calls, idle_marks, specs = [], {}, [], {}

    def switch(at, new):
        nonlocal running, since
        if running is not None and since is not None and at > since:
            spans.append((since, at, specs.get(running, f"#{running}")))
        running, since = new, (at if new is not None else None)

    last = start
    for st in steps:
        at = max(st["ts"], last)
        if st["kind"] == "call":
            calls[st["id"]] = st
            continue
        if st["kind"] != "result":
            continue
        text, name = st["text"], st["name"]
        if name == "wait_event" and st["id"] in calls:
            waits.append((calls[st["id"]]["ts"], st["ts"], "wait"))
            for m in EVENT.finditer(text):
                k = int(m.group(3))
                when = max(_event_wall(int(m.group(1)), st["ts"], offsets, start), since or start)
                if k == running:
                    switch(when, queue.pop() if queue else None)
                elif k in queue:
                    queue.remove(k)
            if "standing idle" in text:
                idle_marks.append(st["ts"])
                queue.clear()
                switch(at, None)
        elif name == "set_goal" and not st["error"]:
            m = re.match(r"ok: goal #(\d+) (.*)", text)
            if m:
                k = int(m.group(1))
                specs[k] = m.group(2).split()[0] if m.group(2) else "?"
                if running is not None:
                    queue.append(running)
                    del queue[:-QUEUE_MAX]
                switch(at, k)
            elif text.startswith("ok: ended"):
                m = re.search(r"active now: #(\d+)", text)
                nxt = int(m.group(1)) if m else None
                if nxt in queue:
                    queue.remove(nxt)
                if nxt is None:
                    queue.clear()
                switch(at, nxt)
        elif name == "observe":
            m = OBS_GOAL.search(text)
            if m and m.group(1) == "none":
                queue.clear()
                switch(at, None)
            elif m and int(m.group(2)) != running:
                k = int(m.group(2))
                specs.setdefault(k, re.search(r"\ngoal #\d+ (\S+)", text).group(1))
                switch(at, k)
        last = at
    switch(end, None)
    clip = lambda xs: [(max(a, start), min(b, end), lab) for a, b, lab in xs
                       if min(b, end) > max(a, start)]
    return clip(spans), clip(waits), idle_marks
