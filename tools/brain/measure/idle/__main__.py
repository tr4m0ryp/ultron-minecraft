"""Idle per minute, night against day, for one brain relay run.

usage: python3 tools/brain/measure/idle <run-dir> [--anchor ISO=TICKS] [--gaps] [--json]
  --anchor  a known overworld clock reading (`time query time` over RCON) when the run has
            no clock.tsv; e.g. 2026-11-14T23:12:01Z=122718
  --gaps    list every no-goal stretch of 20 s or more with what came right before it
"""
import datetime
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from idle import Clock, gaps, load, merge, rates, session_numbers, tally, timeline  # noqa: E402


def hhmm(ms):
    return datetime.datetime.fromtimestamp(ms / 1000, datetime.timezone.utc).strftime("%H:%M:%S")


def before(session, ms):
    """The last goal outcome or set_goal result the thought layer saw before `ms`."""
    seen = [st for st in session["steps"] if st["kind"] == "result" and st["ts"] <= ms
            and st["name"] in ("wait_event", "set_goal") and "goal" in st["text"]]
    return seen[-1]["text"].splitlines()[-1][:110] if seen else "session start"


def main(argv):
    run = argv[0]
    anchor = argv[argv.index("--anchor") + 1] if "--anchor" in argv else None
    sessions = [load(run, n) for n in session_numbers(run)]
    clock = Clock(run, anchor, sessions)
    accs, report = [], {"run": os.path.basename(run.rstrip("/")), "clock": clock.source,
                        "sessions": []}
    for s in sessions:
        spans, waits, idle = timeline(s)
        acc = tally(s, spans, waits, idle, clock)
        accs.append(acc)
        report["sessions"].append({"n": s["n"], "start": hhmm(s["start"]), "phases": rates(acc),
                                   "gaps": [(hhmm(a), round(sec), ph, before(s, a))
                                            for a, sec, ph in gaps(s, spans, clock)]})
    report["total"] = rates(merge(accs))
    if "--json" in argv:
        print(json.dumps(report, indent=1))
        return
    print(f"run {report['run']}, clock from {report['clock']}")
    for s in report["sessions"]:
        print(f"session {s['n']} {s['start']}: {json.dumps(s['phases'])}")
        if "--gaps" in argv:
            for g in s["gaps"]:
                print(f"   gap {g[0]} {g[1]:4d}s {g[2]:7s} after: {g[3]}")
    for p, r in report["total"].items():
        print(f"TOTAL {p:7s} {r['minutes']:5.1f} min: no goal {r['nogoal_spm']:4.1f} s/min, "
              f"in wait_event {r['wait_spm']:4.1f} s/min, waiting with no goal "
              f"{r['idlewait_spm']:4.1f} s/min, idle timeouts {r['idle_timeouts']}")


if __name__ == "__main__":
    main(sys.argv[1:])
