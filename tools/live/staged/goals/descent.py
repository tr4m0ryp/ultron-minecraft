"""Going down a height, live: the shape of the play-run go_to that stalled (start about 9
blocks above a target 12 blocks away, no tools in hand, the edge guard R5 watching the drops).
Cases: ramp (a 1-wide staircase down a 9-high cliff), terraces (3-block drops, 4 wide),
narrow (3-block drops on 1-wide steps), drop3 (one 3-block drop), hut_closed and hut_hole (the goal inside a plank hut), play_canopy
(the play-run terrain copied from the world), sealed (an unreachable goal: expects a failure
plus a TRACE dump, so its trials count as ok when the goal fails).

Usage: python3 descent.py <binary|-> <trials> [case ...]
Site-relative (X0, Z0, Y from kit); the site is restored to plain floor afterwards.
One JSON line per trial; a failed trial carries the bot's flight-recorder TRACE lines.
"""
import json, os, signal, sys
from kit import *
from terrain import clear
import worldcopy


def fill(x0, y0, z0, x1, y1, z1, block="stone"):
    return f"fill {X0 + x0} {Y + y0} {Z0 + z0} {X0 + x1} {Y + y1} {Z0 + z1} {block}"


def go(start, goal, timeout=120):
    prep(at=(X0 + start[0] + 0.5, Y + start[1], Z0 + start[2] + 0.5))
    g = (X0 + goal[0] + 0.5, Y + goal[1], Z0 + goal[2] + 0.5)
    t0 = time.time()
    n = task(f"go_to {g[0]} {g[1]} {g[2]}")
    kind, text, _ = outcome(n, timeout, "go_to")
    p = pos()
    return dict(ok=kind == "goal_done" and dist(p, g) <= 1.5, outcome=kind, text=text,
                secs=round(time.time() - t0, 1), err=dist(p, g))


def ramp():
    rc(fill(4, 1, 10, 16, 9, 22),
       *[fill(17 + i, 1, 10, 17 + i, 8 - i, 10) for i in range(8)])
    return go((6, 10, 16), (27, 1, 24))


def terraces():
    rc(*[fill(4 + 4 * i, 1, 10, 7 + 4 * i, 9 - 3 * i, 22) for i in range(3)])
    return go((5, 10, 16), (20, 1, 16))


def narrow():
    rc(fill(4, 1, 10, 7, 9, 22), fill(8, 1, 10, 8, 6, 22), fill(9, 1, 10, 9, 3, 22))
    return go((5, 10, 16), (16, 1, 16))


def drop3():
    rc(fill(4, 1, 10, 10, 3, 22))
    return go((5, 4, 16), (16, 1, 16))


def hut(roof_hole):
    """The play-run target: inside a small plank hut (2-high walls, roof, chest by the goal)
    whose door gap was never placed, reached from a 9-high plateau (1-wide stair down) with empty hands."""
    rc(fill(2, 1, 12, 8, 9, 20), *[fill(9 + i, 1, 12, 9 + i, 8 - i, 12) for i in range(8)],
       fill(14, 1, 14, 18, 2, 18, "oak_planks"), fill(15, 1, 15, 17, 2, 17, "air"),
       fill(14, 3, 14, 18, 3, 18, "oak_planks"))
    if roof_hole:
        rc(fill(16, 3, 16, 16, 3, 16, "air"))
    rc(fill(15, 1, 15, 15, 1, 15, "chest"))
    return go((4, 10, 16), (16, 1, 16), 150)


def hut_closed():
    return hut(False)


def hut_hole():
    return hut(True)


def sealed():
    """A target inside a bedrock shell: the goal must fail (Unreachable) and the bot must
    print its flight-recorder dump for it."""
    rc(fill(19, 1, 19, 21, 3, 21, "bedrock"), fill(20, 1, 20, 20, 2, 20, "air"))
    return go((4, 1, 20), (20, 1, 20), 90)


def play_canopy():
    """The play-run go_to, replayed on a copy of its terrain (world x -12..16, z -10..12,
    y 67..84 on the site, 30 blocks up): the bot stands on an oak canopy at 7.7 80 -6.5 and
    the target is 8 71 6, 9 blocks down, by the camp hut."""
    worldcopy.replay(rc, (-12, 16, 67, 84, -10, 12), (X0 + 8, Y - 3, Z0 + 10))
    return go((27, 10, 13), (28, 1, 26), 150)


CASES = dict(ramp=ramp, terraces=terraces, narrow=narrow, drop3=drop3, hut_closed=hut_closed,
             hut_hole=hut_hole, play_canopy=play_canopy, sealed=sealed)

if __name__ == "__main__":
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(1))
    binary, trials = sys.argv[1], int(sys.argv[2])
    names = sys.argv[3:] or list(CASES)
    os.environ.setdefault("ULTRON_BRIDGE", "0")
    build_site()
    start(None if binary == "-" else binary, companion=False)
    try:
        for name in names:
            for k in range(trials):
                clear()
                n0 = len(log_lines())
                r = CASES[name]()
                if name == "sealed":
                    dump = trace_lines(n0)
                    r["ok"] = r["outcome"] == "goal_failed" and bool(dump)
                    r["trace_lines"] = len(dump)
                    r["dump_reason"] = next((l for l in dump if "dump" in l.lower()), None)
                if not r["ok"]:
                    r["trace"] = trace_lines(n0)
                print(json.dumps(dict(case=name, trial=k, **r)), flush=True)
                send("goal-end")
                time.sleep(1)
    finally:
        stop()
        clear()
        build_site()
