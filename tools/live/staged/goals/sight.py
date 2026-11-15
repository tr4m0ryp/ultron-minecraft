"""Line of sight for the hands, live: placing a block and sleeping in a bed behind a wall.

The server takes a click on any block within reach, also through a wall. Each wall case has
an open twin, and a third case with a way round the wall.

Cases: place_sealed (target cell inside a closed bedrock shell: must not be placed),
place_round (cell behind a bedrock wall with a way round: placed, and the body ends beyond
the wall), place_open; sleep_sealed (bed inside a closed bedrock shell at night: must not be
slept in), sleep_round (bed behind a wall with a way round: slept in, the last position
awake beyond the wall), sleep_open. Block, sleeping state and positions are read over RCON.

Usage: python3 sight.py [-n trials] [--binary PATH] [case ...]
One JSON line per trial and a summary line per case; the site is cleared at the end.
"""
import argparse, json, re, sys
from kit import *

CELL = (CX + 3, Y + 1, CZ)
WALL_X = CX + 2
START = (WALL_X - 0.3, Y + 1, CZ + 0.5)  # touching the wall's west face
BED = "red_bed"


def clear():
    rc(f"fill {CX - 3} {Y + 1} {CZ - 6} {CX + 9} {Y + 5} {CZ + 6} air",
       f"fill {CX - 3} {Y - 3} {CZ - 6} {CX + 9} {Y} {CZ + 6} stone",
       f"kill @e[type=item,x={CX},y={Y},z={CZ},distance=..40]")


def shell(x1):
    """Closed bedrock box around the cells x CX+3..x1 at z CZ, two high inside."""
    rc(f"fill {WALL_X} {Y + 1} {CZ - 1} {x1 + 1} {Y + 3} {CZ + 1} bedrock",
       f"fill {CX + 3} {Y + 1} {CZ} {x1} {Y + 2} {CZ} air")


def wall():
    """Bedrock wall across the way, five wide and three high, open at both ends."""
    rc(f"fill {WALL_X} {Y + 1} {CZ - 2} {WALL_X} {Y + 3} {CZ + 2} bedrock")


def bed():
    rc(f"setblock {CX + 3} {Y + 1} {CZ} {BED}[facing=east,part=foot]",
       f"setblock {CX + 4} {Y + 1} {CZ} {BED}[facing=east,part=head]")


def behind_wall(p):
    """West of the wall and within its width: no line to anything right behind it."""
    return bool(p) and p[0] < WALL_X and CZ - 2 <= p[2] < CZ + 3


def place(kind):
    clear()
    {"sealed": lambda: shell(CX + 3), "round": wall, "open": lambda: None}[kind]()
    prep(items=[("hotbar.1", "cobblestone", 8)], at=START, comp_at=(CX - 12.5, Y + 1, CZ - 12.5))
    t0 = time.time()
    n = task("place cobblestone %d %d %d" % CELL)
    out, text, _ = outcome(n, 90, "place")
    time.sleep(1)
    placed, p = is_block(*CELL, "cobblestone"), pos()
    through = placed and kind != "open" and behind_wall(p)
    ok = {"sealed": not placed, "round": placed and not through, "open": placed}[kind]
    return dict(ok=ok, placed=placed, through_wall=through, outcome=out, text=text, pos=p,
                secs=round(time.time() - t0, 1))


def asleep():
    return bool(re.search(r"\[I;", rc(f"data get entity {NAME} sleeping_pos")))


def sleep(kind):
    clear()
    {"sealed": lambda: shell(CX + 4), "round": wall, "open": lambda: None}[kind]()
    bed()
    prep(night=True, at=START, comp_at=(CX - 12.5, Y + 1, CZ - 12.5))
    t0 = time.time()
    n = task("sleep_in_bed")
    last, slept, out = pos(), False, None
    while time.time() - t0 < 60 and not slept and out is None:
        time.sleep(0.2)
        p = pos()
        slept = asleep()
        if not slept:
            last = p or last
            wakes = [m.groups() for l in log_lines()[n:] if (m := WAKE_RE.match(l))]
            out = next(((k, t) for _, k, s, _, t in wakes
                        if s == "sleep_in_bed" and k in ("goal_done", "goal_failed")), None)
    time.sleep(1.0)
    slept = slept or asleep()
    kind_, text, _ = outcome(n, 5, "sleep_in_bed")
    through = slept and kind != "open" and behind_wall(last)
    rc(f"fill {CX + 3} {Y + 1} {CZ} {CX + 4} {Y + 1} {CZ} air", "time set noon")
    ok = {"sealed": not slept, "round": slept and not through, "open": slept}[kind]
    return dict(ok=ok, slept=slept, through_wall=through, last_awake_pos=last, outcome=kind_,
                text=text, secs=round(time.time() - t0, 1))


CASES = dict(place_sealed=lambda: place("sealed"), place_round=lambda: place("round"),
             place_open=lambda: place("open"), sleep_sealed=lambda: sleep("sealed"),
             sleep_round=lambda: sleep("round"), sleep_open=lambda: sleep("open"))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-n", type=int, default=5)
    ap.add_argument("--binary", default=None)
    ap.add_argument("cases", nargs="*")
    a = ap.parse_args()
    build_site()
    start(binary=a.binary)
    try:
        for name in a.cases or list(CASES):
            wins = 0
            for i in range(a.n):
                r = CASES[name]()
                wins += r["ok"]
                print(json.dumps(dict(r, case=name, trial=i + 1)), flush=True)
            print(json.dumps(dict(summary=name, ok=wins, of=a.n)), flush=True)
    finally:
        clear()
        stop()


if __name__ == "__main__":
    sys.exit(main())
