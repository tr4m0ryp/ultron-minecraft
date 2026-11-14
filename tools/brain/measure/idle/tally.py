"""Splits a session's wall time into night and day and sums idle per phase.

Per phase (night, day, unknown when the clock cannot tell):
  minutes      wall minutes of the session in that phase
  nogoal_spm   seconds per minute with no running goal
  wait_spm     seconds per minute blocked in wait_event
  idlewait_spm seconds per minute blocked in wait_event with no running goal
  idle_timeouts wait_event timeouts that reported an idle body
"""
STEP_MS = 250
PHASES = ("night", "day", "unknown")


def _covered(spans, ms):
    return any(a <= ms < b for a, b, _ in spans)


def phase_of(clock, ms):
    n = clock.night(ms)
    return "unknown" if n is None else ("night" if n else "day")


def tally(session, spans, waits, idle_marks, clock):
    acc = {p: {"ms": 0, "nogoal": 0, "wait": 0, "idlewait": 0, "idle_timeouts": 0}
           for p in PHASES}
    ms = session["start"]
    while ms < session["end"]:
        a = acc[phase_of(clock, ms)]
        busy = _covered(spans, ms)
        waiting = _covered(waits, ms)
        a["ms"] += STEP_MS
        a["nogoal"] += 0 if busy else STEP_MS
        a["wait"] += STEP_MS if waiting else 0
        a["idlewait"] += STEP_MS if waiting and not busy else 0
        ms += STEP_MS
    for t in idle_marks:
        acc[phase_of(clock, t)]["idle_timeouts"] += 1
    return acc


def rates(acc):
    out = {}
    for p, a in acc.items():
        if not a["ms"]:
            continue
        minutes = a["ms"] / 60000
        out[p] = {"minutes": round(minutes, 1),
                  "nogoal_spm": round(a["nogoal"] / 1000 / minutes, 1),
                  "wait_spm": round(a["wait"] / 1000 / minutes, 1),
                  "idlewait_spm": round(a["idlewait"] / 1000 / minutes, 1),
                  "idle_timeouts": a["idle_timeouts"]}
    return out


def merge(accs):
    total = {p: {"ms": 0, "nogoal": 0, "wait": 0, "idlewait": 0, "idle_timeouts": 0}
             for p in PHASES}
    for acc in accs:
        for p in PHASES:
            for k in total[p]:
                total[p][k] += acc[p][k]
    return total


def gaps(session, spans, clock, min_s=20):
    """No-goal stretches of at least `min_s` seconds: (start ms, seconds, phase)."""
    edges = sorted((a, b) for a, b, _ in spans)
    out, cur = [], session["start"]
    for a, b in edges + [(session["end"], session["end"])]:
        if a - cur >= min_s * 1000:
            out.append((cur, (a - cur) / 1000, phase_of(clock, cur)))
        cur = max(cur, b)
    return out
