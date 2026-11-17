"""Filling empty bottles at water, live: the step brewing from scratch begins with.

Usage: python3 brewing/bottles.py <binary|-> <trials> [case ...]

A one-block water source is set into the floor 5 blocks from the body. Counts are read on
the server, potions by their contents.
  fill       3 glass bottles, `craft water_potion 3`: 3 water bottles, 0 empty ones
  from_glass 3 glass and a crafting table to put down: bottles crafted, then filled
  awkward    1 glass bottle, nether wart, blaze powder and a brewing stand to put down,
             `craft awkward_potion 1`: filled, then brewed
  no_water   3 glass bottles and no water within 16 blocks: one failure with the reason,
             the bottles kept
"""
import json, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from kit import *

POOL = (CX + 5, Y, CZ)
STATIONS = ("crafting_table", "brewing_stand")
NO_WATER = ("cannot craft water_potion: no water source within 16 blocks to fill 3 glass_bottle at. "
            "Go to water, or carry a water_bucket and pour it out.")


def potion(kind):
    return 'potion[potion_contents={potion:"minecraft:%s"}]' % kind


def have(item):
    n = count_item(NAME, item)
    return n if isinstance(n, int) else -1


def site(pool=True):
    rc(f"fill {X0} {Y - 3} {Z0} {X1} {Y} {Z1} stone", f"fill {X0} {Y + 1} {Z0} {X1} {Y + 6} {Z0 + 20} air",
       f"fill {X0} {Y + 1} {Z0 + 21} {X1} {Y + 6} {Z1} air",
       f"kill @e[type=item,x={CX},y={Y},z={CZ},distance=..40]")
    if pool:
        rc("setblock %d %d %d water" % POOL)


def goal(spec, timeout):
    n = task(spec)
    kind, text, wakes = outcome(n, timeout, "craft")
    time.sleep(2)
    return kind, text, sum(1 for w in wakes if w[1] == "goal_failed")


def fill():
    site()
    prep(items=[("hotbar.1", "glass_bottle", 3)])
    kind, text, _ = goal("craft water_potion 3", 90)
    water, empty = have(potion("water")), have("glass_bottle")
    return dict(ok=kind == "goal_done" and water == 3 and empty == 0, outcome=kind, text=text,
                water_bottles=water, glass_bottles=empty, pool_left=is_block(*POOL, "water"))


def from_glass():
    site()
    prep(items=[("hotbar.1", "glass", 3), ("hotbar.2", "crafting_table", 1)])
    kind, text, _ = goal("craft water_potion 3", 150)
    water, glass = have(potion("water")), have("glass")
    return dict(ok=kind == "goal_done" and water == 3 and glass == 0, outcome=kind, text=text,
                water_bottles=water, glass=glass)


def awkward():
    site()
    prep(items=[("hotbar.1", "glass_bottle", 1), ("hotbar.2", "nether_wart", 1), ("hotbar.3", "blaze_powder", 1),
                ("hotbar.4", "brewing_stand", 1)])
    kind, text, _ = goal("craft awkward_potion 1", 180)
    got, empty = have(potion("awkward")), have("glass_bottle")
    return dict(ok=kind == "goal_done" and got == 1 and empty == 0, outcome=kind, text=text,
                awkward_potions=got, glass_bottles=empty)


def no_water():
    site(pool=False)
    prep(items=[("hotbar.1", "glass_bottle", 3)])
    kind, text, fails = goal("craft water_potion 3", 60)
    water, empty = have(potion("water")), have("glass_bottle")
    return dict(ok=kind == "goal_failed" and text == NO_WATER and fails == 1 and water == 0 and empty == 3,
                outcome=kind, failures=fails, reason_matches=text == NO_WATER,
                text=None if text == NO_WATER else text, water_bottles=water, glass_bottles=empty)


CASES = dict(fill=fill, from_glass=from_glass, awkward=awkward, no_water=no_water)

if __name__ == "__main__":
    binary = None if sys.argv[1] == "-" else sys.argv[1]
    names = sys.argv[3:] or list(CASES)
    build_site()
    start(binary, companion=False)
    try:
        for name in names:
            wins = 0
            for i in range(int(sys.argv[2])):
                r = CASES[name]()
                wins += r["ok"]
                print(json.dumps(dict(case=name, trial=i + 1, **r)), flush=True)
            print(f"SUMMARY {name}: {wins}/{sys.argv[2]}", flush=True)
    finally:
        site(pool=False)
        for b in STATIONS:
            rc(f"fill {X0} {Y + 1} {Z0} {X1} {Y + 3} {Z1} air replace minecraft:{b}")
        stop()
