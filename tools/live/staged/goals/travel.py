"""Travel goals live: go_to across terrain, follow and come_here with the companion moving
around obstacles and up/down terrain, stay. Usage: python3 travel.py [case ...]"""
import json, sys
from kit import *

WALL_Z = 1020


def terrain():
    """A 3-high wall across the site at z=1020 with a gap at x 1436..1440, and a plateau
    3 blocks up at x 1428..1440, z 1030..1040 reached by a 1-block staircase from the west."""
    build_site()
    rc(f"fill {X0} {Y + 1} {WALL_Z} 1435 {Y + 3} {WALL_Z} stone",
       f"fill 1428 {Y + 1} 1030 {X1} {Y + 3} {Z1} stone",
       f"fill 1425 {Y + 1} 1033 1425 {Y + 1} 1036 stone",
       f"fill 1426 {Y + 1} 1033 1426 {Y + 2} 1036 stone",
       f"fill 1427 {Y + 1} 1033 1427 {Y + 3} 1036 stone")


def go_to():
    # The companion waits at the destination so the leash does not pull the bot off it.
    prep(at=(1405.5, Y + 1, 1005.5), comp_at=(1436.5, Y + 4, 1038.5))
    goal = (1434.5, Y + 4, 1037.5)
    n = task(f"go_to {goal[0]} {goal[1]} {goal[2]}")
    kind, text, wakes = outcome(n, 90, "go_to")
    p = pos()
    return dict(case="go_to", outcome=kind, text=text, server_pos=p, err=dist(p, goal))


def follow():
    prep(at=(1405.5, Y + 1, 1005.5), comp_at=(1410.5, Y + 1, 1005.5))
    n = task("follow companion 3")
    legs = []
    for c in [(1425.5, Y + 1, 1010.5), (1438.5, Y + 1, 1025.5), (1433.5, Y + 4, 1037.5),
              (1415.5, Y + 1, 1032.5), (1404.5, Y + 1, 1012.5)]:
        rc(f"tp {COMP} {c[0]} {c[1]} {c[2]}")
        best = None
        for _ in range(60):
            time.sleep(0.5)
            d = dist(pos(), pos(COMP))
            best = d if best is None else min(best, d)
            if d is not None and d <= 4.5:
                break
        legs.append(dict(comp=c, dist=d, best=best, bot=pos()))
    wakes = [w[1:] for w in outcome(n, 0)[2]]
    return dict(case="follow", legs=legs, reached=sum(1 for l in legs if l["dist"] <= 4.5),
                of=len(legs), wakes=wakes[-4:])


def come_here():
    prep(at=(1405.5, Y + 1, 1005.5), comp_at=(1433.5, Y + 4, 1037.5))
    n = task("come_here")
    kind, text, _ = outcome(n, 90, "come_here")
    return dict(case="come_here", outcome=kind, text=text, dist=dist(pos(), pos(COMP)))


def stay():
    """Holds its spot while the companion walks off, and walks back when displaced."""
    prep(at=(1410.5, Y + 1, 1010.5), comp_at=(1412.5, Y + 1, 1010.5))
    n = task("stay")
    time.sleep(1)
    start = pos()
    rc(f"tp {COMP} 1438.5 {Y + 1} 1005.5")
    time.sleep(8)
    held = dist(start, pos())
    rc(f"tp {NAME} 1414.5 {Y + 1} 1013.5")
    time.sleep(8)
    kind, text, wakes = outcome(n, 0)
    return dict(case="stay", moved_while_companion_left=held,
                back_after_displaced=dist(start, pos()), outcome=kind)


CASES = dict(go_to=go_to, follow=follow, come_here=come_here, stay=stay)

if __name__ == "__main__":
    names = sys.argv[1:] or list(CASES)
    terrain()
    start()
    try:
        for name in names:
            print(json.dumps(CASES[name]()), flush=True)
    finally:
        stop()
