"""Crafting goals live: the craft planner from raw kits, with no table or furnace at the site.
Each case starts from a fixed inventory, sets one `craft` goal and verifies the result over
RCON: the item count in the server inventory and every crafting table or furnace the bot put
down (counted and removed with `fill ... replace`). The impossible case must fail once with
the exact shopping list. The smelt case runs the `smelt` goal, furnace crafted from the kit.

Usage: crafting.py [--trials N] [--binary PATH] [case ...]
Prints one JSON line per trial and a summary line per case.
"""
import argparse, json, re, sys
from kit import *

COMP_AT = (CX - 3.5, Y + 1, CZ - 3.5)
BOX = (X0, Y + 1, Z0, X1, Y + 12, Z1)
FILLED_RE = re.compile(r"filled (\d+) block")

# (goal spec, kit [(slot, item, n)], item checked, count wanted, timeout s)
CASES = {
    "stone_pickaxe": ("craft stone_pickaxe 1",
                      [("hotbar.1", "oak_log", 2), ("hotbar.2", "cobblestone", 3)],
                      "stone_pickaxe", 1, 90),
    "iron_pickaxe": ("craft iron_pickaxe 1",
                     [("hotbar.1", "raw_iron", 3), ("hotbar.2", "coal", 1),
                      ("hotbar.3", "oak_log", 3), ("hotbar.4", "cobblestone", 8)],
                     "iron_pickaxe", 1, 180),
    "chest": ("craft chest 1", [("hotbar.1", "oak_log", 3)], "chest", 1, 90),
    "door": ("craft oak_door 3", [("hotbar.1", "oak_log", 3)], "oak_door", 3, 90),
    "bed": ("craft white_bed 1", [("hotbar.1", "white_wool", 3), ("hotbar.2", "oak_log", 2)],
            "white_bed", 1, 90),
    "bread": ("craft bread 2", [("hotbar.1", "wheat", 6), ("hotbar.2", "oak_log", 1)],
              "bread", 2, 90),
    "torches": ("craft torch 4", [("hotbar.1", "coal", 1), ("hotbar.2", "oak_log", 1)],
                "torch", 4, 60),
    "smelt": ("smelt iron_ingot 2",
              [("hotbar.1", "raw_iron", 2), ("hotbar.2", "coal", 1),
               ("hotbar.3", "oak_log", 1), ("hotbar.4", "cobblestone", 8)],
              "iron_ingot", 2, 120),
    "dyed_box": ("craft black_shulker_box 1",
                 [("hotbar.1", "shulker_box", 1), ("hotbar.2", "black_dye", 1)],
                 "black_shulker_box", 1, 60),
    "impossible": ("craft stone_pickaxe 1", [("hotbar.1", "cobblestone", 1)],
                   "stone_pickaxe", 0, 30),
}

# The one failure text the impossible case must return, word for word.
SHOPPING = ("cannot craft stone_pickaxe: need 1 crafting_table, have 0: craft 1 from 4 any plank; "
            "need 4 any plank, have 0: craft 4 from 1 any log; need 1 any log, have 0; "
            "need 3 cobblestone/blackstone/cobbled_deepslate, have 1; "
            "need 2 stick, have 0: craft 4 from 2 any plank; "
            "need 2 any plank, have 0: craft 4 from 1 any log; need 1 any log, have 0. "
            "Get 2 any log, 2 cobblestone/blackstone/cobbled_deepslate.")


def site():
    """This worker's site only: floor, empty air above, no gamerules or clock changes."""
    rc(f"forceload add {X0 - 16} {Z0 - 16} {X1 + 16} {Z1 + 16}",
       f"fill {X0} {Y - 3} {Z0} {X1} {Y} {Z1} stone")
    clear_site()


def clear_site():
    """Removes stations and anything else above the floor; returns the stations it removed."""
    x0, y0, z0, x1, y1, z1 = BOX
    found = {}
    for block in ("crafting_table", "furnace"):
        out = rc(f"fill {x0} {y0} {z0} {x1} {y1} {z1} air replace minecraft:{block}")
        m = FILLED_RE.search(out)
        found[block] = int(m.group(1)) if m else 0
    zm = (z0 + z1) // 2
    rc(f"fill {x0} {y0} {z0} {x1} {y1} {zm} air", f"fill {x0} {y0} {zm + 1} {x1} {y1} {z1} air",
       f"kill @e[type=item,x={CX},y={Y},z={CZ},distance=..40]")
    return found


def trial(name):
    spec, items, check, want, timeout = CASES[name]
    clear_site()
    prep(items=items, comp_at=COMP_AT)
    n = task(spec)
    kind, text, _ = outcome(n, timeout, spec.split()[0])
    time.sleep(2)
    got = count_item(NAME, check)
    got = got if isinstance(got, int) else -1
    stations = clear_site()
    if name == "impossible":
        ok = kind == "goal_failed" and text == SHOPPING and got == 0
    else:
        ok = kind == "goal_done" and got >= want
    return dict(case=name, ok=ok, outcome=kind, text=text, server_count=got,
                stations_placed=stations)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--trials", type=int, default=5)
    ap.add_argument("--binary", default=None)
    ap.add_argument("cases", nargs="*")
    a = ap.parse_args()
    names = a.cases or list(CASES)
    site()
    start(binary=a.binary)
    try:
        for name in names:
            wins = 0
            for i in range(a.trials):
                r = trial(name)
                wins += r["ok"]
                print(json.dumps(dict(r, trial=i + 1)), flush=True)
            print(json.dumps(dict(summary=name, ok=wins, trials=a.trials)), flush=True)
    finally:
        clear_site()
        stop()


if __name__ == "__main__":
    sys.exit(main())
