"""Movement that changes the world, live: a sealed pocket, a 6-deep pit, a 4-block gap, a
6-high ledge, a 3-thick wall, and the companion across broken terrain (come_here, give).

Usage: python3 terrain.py <binary|-> <trials> [case ...]
Each trial rebuilds its structure on the goal site (x 1400..1440, z 1000..1040, y 100),
gives the bot an iron pickaxe and 32 dirt, sets one goal and checks the result over RCON.
The site is restored to its plain floor at the end. One JSON line per trial.
"""
import json, signal, sys
from kit import *

KIT = [("hotbar.0", "iron_pickaxe", 1), ("hotbar.1", "dirt", 32)]
TOP = Y + 16


def clear():
    """The plain site: its 4-thick floor, open air above it, below it and on either side
    of it where the walls run past the floor."""
    rc(f"fill {X0} {Y + 1} 985 {X1} {TOP} 1055 air",
       f"fill {X0} {Y - 4} 985 {X1} {Y} {Z0 - 1} air",
       f"fill {X0} {Y - 4} {Z1 + 1} {X1} {Y} 1055 air",
       f"fill {X0} {Y - 4} {Z0} {X1} {Y - 4} {Z1} air",
       f"fill {X0} {Y - 3} {Z0} {X1} {Y} {Z1} stone",
       f"kill @e[type=item,x={CX},y={Y},z={CZ},distance=..80]")


def run(spec, label, timeout):
    t0 = time.time()
    n = task(spec)
    kind, text, _ = outcome(n, timeout, label)
    return kind, text, round(time.time() - t0, 1)


def pocket():
    rc(f"fill 1404 {Y + 1} 1004 1416 {Y + 10} 1016 stone",
       f"fill 1410 {Y + 4} 1010 1411 {Y + 5} 1010 air")
    prep(items=KIT, at=(1410.5, Y + 4, 1010.5))
    goal = (1430.5, Y + 1, 1025.5)
    kind, text, secs = run(f"go_to {goal[0]} {goal[1]} {goal[2]}", "go_to", 150)
    p = pos()
    return dict(ok=kind == "goal_done" and dist(p, goal) <= 1.5, outcome=kind, secs=secs,
                err=dist(p, goal), text=text)


def pocket_idle():
    """No goal at all: the escape drive must get the body out under the sky."""
    rc(f"fill 1404 {Y + 1} 1004 1416 {Y + 10} 1016 stone",
       f"fill 1410 {Y + 4} 1010 1411 {Y + 5} 1010 air")
    prep(items=KIT, at=(1410.5, Y + 4, 1010.5))
    t0, p = time.time(), None
    while time.time() - t0 < 120:
        time.sleep(2)
        p = pos()
        if p and (p[1] >= Y + 11 or not (1404 <= p[0] < 1417 and 1004 <= p[2] < 1017)):
            return dict(ok=True, secs=round(time.time() - t0, 1), pos=p)
    return dict(ok=False, secs=None, pos=p)


def pit():
    rc(f"fill 1404 {Y + 1} 1004 1416 {Y + 6} 1016 stone",
       f"fill 1410 {Y + 1} 1010 1410 {Y + 6} 1010 air")
    prep(items=KIT, at=(1410.5, Y + 1, 1010.5))
    goal = (1414.5, Y + 7, 1014.5)
    kind, text, secs = run(f"go_to {goal[0]} {goal[1]} {goal[2]}", "go_to", 120)
    p = pos()
    return dict(ok=kind == "goal_done" and dist(p, goal) <= 1.5, outcome=kind, secs=secs,
                err=dist(p, goal), text=text)


def gap():
    rc(f"fill 1400 {Y + 10} 1022 1405 {Y + 10} 1038 stone",
       f"fill 1410 {Y + 10} 1022 1415 {Y + 10} 1038 stone")
    prep(items=KIT, at=(1402.5, Y + 11, 1030.5))
    goal = (1413.5, Y + 11, 1030.5)
    kind, text, secs = run(f"go_to {goal[0]} {goal[1]} {goal[2]}", "go_to", 120)
    p = pos()
    return dict(ok=kind == "goal_done" and dist(p, goal) <= 1.5, outcome=kind, secs=secs,
                err=dist(p, goal), text=text)


def ledge():
    rc(f"fill 1425 {Y + 1} 1020 1435 {Y + 6} 1030 stone")
    prep(items=KIT, at=(1420.5, Y + 1, 1025.5))
    goal = (1430.5, Y + 7, 1025.5)
    kind, text, secs = run(f"go_to {goal[0]} {goal[1]} {goal[2]}", "go_to", 120)
    p = pos()
    return dict(ok=kind == "goal_done" and dist(p, goal) <= 1.5, outcome=kind, secs=secs,
                err=dist(p, goal), text=text)


def wall():
    rc(f"fill 1420 {Y - 4} 985 1422 {TOP} 1055 stone")
    # Stone ground on either side of the site too, so walking around takes 35+ blocks.
    rc(f"fill {X0} {Y - 3} 985 {X1} {Y} {Z0 - 1} stone",
       f"fill {X0} {Y - 3} {Z1 + 1} {X1} {Y} 1055 stone")
    prep(items=KIT, at=(1410.5, Y + 1, 1020.5))
    goal = (1430.5, Y + 1, 1020.5)
    kind, text, secs = run(f"go_to {goal[0]} {goal[1]} {goal[2]}", "go_to", 120)
    p = pos()
    return dict(ok=kind == "goal_done" and dist(p, goal) <= 1.5, outcome=kind, secs=secs,
                err=dist(p, goal), text=text)


def broken():
    """A 4-deep ditch, a 4-high wall and a mound between the bot and the companion, 38
    blocks apart."""
    rc(f"fill 1415 {Y - 3} {Z0} 1416 {Y} {Z1} air",
       f"fill 1415 {Y - 4} {Z0} 1416 {Y - 4} {Z1} stone",
       f"fill 1422 {Y + 1} {Z0} 1423 {Y + 4} {Z1} stone",
       f"fill 1428 {Y + 1} 1026 1432 {Y + 3} 1030 dirt")
    prep(items=KIT, at=(1408.5, Y + 1, 1008.5), comp_at=(1435.5, Y + 1, 1035.5))
    rc(f"gamemode survival {COMP}", f"clear {COMP}")


def come():
    broken()
    kind, text, secs = run("come_here", "come_here", 150)
    d = dist(pos(), pos(COMP))
    return dict(ok=kind == "goal_done", outcome=kind, secs=secs, dist=d, text=text)


def give():
    broken()
    rc(f"item replace entity {NAME} hotbar.2 with oak_planks 8")
    kind, text, secs = run("give companion oak_planks 3", "give", 150)
    time.sleep(3)
    got = count_item(COMP, "oak_planks")
    return dict(ok=kind == "goal_done" and got == 3, outcome=kind, secs=secs,
                companion_planks=got, text=text)


CASES = dict(pocket=pocket, pocket_idle=pocket_idle, pit=pit, gap=gap, ledge=ledge,
             wall=wall, come=come, give=give)

if __name__ == "__main__":
    # A batch started in the background ignores SIGINT; TERM still restores the site.
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(1))
    binary, trials = sys.argv[1], int(sys.argv[2])
    names = sys.argv[3:] or list(CASES)
    build_site()
    rc("gamerule spawn_mobs false", "gamerule advance_time false", "time set noon")
    start(None if binary == "-" else binary)
    try:
        for name in names:
            for k in range(trials):
                clear()
                r = CASES[name]()
                print(json.dumps(dict(case=name, trial=k, **r)), flush=True)
                send("goal-end")
                time.sleep(1)
    finally:
        stop()
        clear()
        build_site()
        rc("gamerule advance_time true", "gamerule spawn_mobs true")
