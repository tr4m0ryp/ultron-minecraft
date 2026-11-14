"""The overworld game clock for the play relay, read over RCON (tools/rcon.py, password
from the repo .env, never printed) and kept as samples in <run>/clock.tsv.

usage: gameclock.py sample <run-dir>   append `<unix ms> <ticks>` (silent, exit 1 on failure)
       gameclock.py now <run-dir>      one line for the kickoff: day, phase, time of day
       gameclock.py notice <run-dir>   a line when day turned to dusk, night or dawn since
                                       the last notice (state in <run>/phase.said), else
                                       nothing
       gameclock.py stamp <run-dir>    `day D, <phase>` for the handover note
Between samples the clock runs at 20 ticks per second. 0 is sunrise; night is 13000..23000.
"""
import os
import re
import sys
import time

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))

DAY = 24000
TICKS_PER_S = 20
STALE_S = 600
PHASES = ((0, "day"), (12000, "dusk"), (13000, "night"), (23000, "dawn"))


def query():
    """Total overworld clock ticks, or None when the server cannot be asked."""
    try:
        import rcon

        r = rcon.connect()
        m = re.search(r"Clock minecraft:overworld is at (\d+)", r.cmd("time query time"))
        if m:
            return int(m.group(1))
        # Servers before the world clocks answer `daytime` and `day` separately.
        tod = re.search(r"The time is (\d+)", r.cmd("time query daytime"))
        day = re.search(r"The time is (\d+)", r.cmd("time query day"))
        if tod and day:
            return int(day.group(1)) * DAY + int(tod.group(1)) % DAY
    except (OSError, SystemExit, KeyError, ValueError, ConnectionError):
        return None
    return None


def latest(run):
    """Clock ticks now, extrapolated from the last sample, or None without a fresh one."""
    try:
        lines = open(os.path.join(run, "clock.tsv")).read().split("\n")
    except OSError:
        return None
    rows = [x.split() for x in lines if x.strip()]
    if not rows:
        return None
    at, ticks = int(rows[-1][0]), int(rows[-1][1])
    age = time.time() - at / 1000
    if age > STALE_S:
        return None
    return ticks + int(max(age, 0) * TICKS_PER_S)


def phase(ticks):
    tod = ticks % DAY
    return [name for start, name in PHASES if tod >= start][-1]


def describe(ticks):
    tod = ticks % DAY
    p = phase(ticks)
    nxt = {"day": 12000, "dusk": 13000, "night": 23000, "dawn": DAY}[p]
    eta = round((nxt - tod) / TICKS_PER_S)
    then = {"day": "dusk", "dusk": "night", "night": "dawn", "dawn": "day"}[p]
    return f"day {ticks // DAY + 1}, {p} (time {tod} of {DAY}; {then} in about {eta} s)"


NOTICE = {
    "dusk": "Game clock: dusk, night falls in about {eta} s. Finish or move work to the base "
            "now, arm a retreat, and plan night work (see AT NIGHT).",
    "night": "Game clock: night has fallen. Work from AT NIGHT, sleep if a bed is near, and "
             "keep a goal running.",
    "dawn": "Game clock: dawn, day in about {eta} s. Outdoor projects are safe again soon.",
    "day": "Game clock: it is day. Outdoor work is safe.",
}


def notice(run):
    ticks = latest(run)
    if ticks is None:
        return ""
    p = phase(ticks)
    said = os.path.join(run, "phase.said")
    try:
        before = open(said).read().strip()
    except OSError:
        before = ""
    if before == p:
        return ""
    with open(said, "w") as f:
        f.write(p)
    if not before:
        return ""  # the kickoff already told the phase
    nxt = {"day": 12000, "dusk": 13000, "night": 23000, "dawn": DAY}[p]
    return NOTICE[p].format(eta=round((nxt - ticks % DAY) / TICKS_PER_S))


def main(argv):
    cmd, run = argv[0], argv[1]
    if cmd == "sample":
        ticks = query()
        if ticks is None:
            return 1
        with open(os.path.join(run, "clock.tsv"), "a") as f:
            f.write(f"{int(time.time() * 1000)} {ticks}\n")
        return 0
    ticks = latest(run)
    if cmd == "now":
        print(describe(ticks) if ticks is not None else "unknown (no game clock)")
        if ticks is not None:
            with open(os.path.join(run, "phase.said"), "w") as f:
                f.write(phase(ticks))
    elif cmd == "stamp":
        print(f"day {ticks // DAY + 1}, {phase(ticks)}" if ticks is not None else "day unknown")
    elif cmd == "notice":
        print(notice(run))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
