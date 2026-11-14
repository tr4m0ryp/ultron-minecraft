"""The game clock over wall time, so each moment of a run can be called night or day.

Sources, best first:
  clock.tsv in the run dir: `<unix ms> <overworld clock ticks>` lines the relay samples
    over RCON; between samples the clock advances 20 ticks per second.
  an anchor `<ISO time>=<ticks>` given on the command line: one known reading, assumed to
    run at 20 ticks per second back and forward (true while a player is online and nobody
    sets the time).
  the night flag in observe results, held until the next observe. The body's flag froze at
    its login value in the runs of 2026-11-14, so this source is reported as unreliable.
"""
import datetime
import os
import re

DAY = 24000
NIGHT = (13000, 23000)
TICKS_PER_MS = 20 / 1000


def is_night_ticks(ticks):
    return NIGHT[0] <= ticks % DAY < NIGHT[1]


def parse_anchor(text):
    when, ticks = text.rsplit("=", 1)
    dt = datetime.datetime.fromisoformat(when.replace("Z", "+00:00"))
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=datetime.timezone.utc)
    return int(dt.timestamp() * 1000), int(ticks)


def samples(run):
    path = os.path.join(run, "clock.tsv")
    out = []
    if os.path.exists(path):
        for line in open(path):
            parts = line.split()
            if len(parts) >= 2 and parts[0].isdigit() and parts[1].isdigit():
                out.append((int(parts[0]), int(parts[1])))
    return sorted(out)


class Clock:
    """`ticks(ms)` gives the clock reading or None; `night(ms)` gives True, False or None."""

    def __init__(self, run, anchor=None, sessions=()):
        self.points = samples(run)
        self.source = "clock.tsv" if self.points else None
        if not self.points and anchor:
            self.points = [parse_anchor(anchor)]
            self.source = "anchor"
        self.flags = []
        if not self.points:
            self.source = "observe flag (unreliable)"
            for s in sessions:
                for st in s["steps"]:
                    if st["kind"] == "result" and st["name"] == "observe":
                        m = re.search(r" (night|day) mode ", st["text"])
                        if m:
                            self.flags.append((st["ts"], m.group(1) == "night"))
            self.flags.sort()

    def ticks(self, ms):
        if not self.points:
            return None
        before = [p for p in self.points if p[0] <= ms]
        at, t = before[-1] if before else self.points[0]
        value = t + (ms - at) * TICKS_PER_MS
        return int(value) if value >= 0 else None

    def night(self, ms):
        t = self.ticks(ms)
        if t is not None:
            return is_night_ticks(t)
        if self.points:
            return None
        before = [f for f in self.flags if f[0] <= ms]
        flag = before[-1] if before else (self.flags[0] if self.flags else None)
        return flag[1] if flag else None
