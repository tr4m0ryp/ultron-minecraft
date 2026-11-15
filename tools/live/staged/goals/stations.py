"""Brewing stand and smithing table live: the craft planner over the 291 recipes of those
two stations, from fixed kits with no station at the site. Every result is read over RCON:
the item counts in the server inventory (potions by their contents) and the stations the bot
put down (counted and removed with `fill ... replace`).

Cases: netherite (a diamond pickaxe upgraded with a template and an ingot, at a smithing
table crafted from the kit), brew_two (an awkward potion, then a potion of healing from it,
two goals on one blaze powder), brew_chain (three water bottles to three potions of healing
in one goal), no_upgrade and no_potion (impossible: one failure, the shopping list word for
word).

Usage: stations.py [--trials N] [--binary PATH] [case ...]
Prints one JSON line per trial and a summary line per case.
"""
import argparse, json, re, sys
from kit import *

COMP_AT = (CX - 3.5, Y + 1, CZ - 3.5)
BOX = (X0, Y + 1, Z0, X1, Y + 12, Z1)
FILLED_RE = re.compile(r"filled (\d+) block")
STATIONS = ("crafting_table", "furnace", "brewing_stand", "smithing_table")


def potion(kind):
    return 'potion[potion_contents={potion:"minecraft:%s"}]' % kind


SMITH_KIT = [("hotbar.1", "diamond_pickaxe", 1), ("hotbar.2", "netherite_upgrade_smithing_template", 1),
             ("hotbar.3", "netherite_ingot", 1), ("hotbar.4", "iron_ingot", 2), ("hotbar.5", "oak_log", 2)]
BREW_KIT = [("hotbar.1", "nether_wart", 1), ("hotbar.2", "glistering_melon_slice", 1),
            ("hotbar.3", "blaze_powder", 1), ("hotbar.4", "brewing_stand", 1)]
WATER = [("inventory.0", potion("water"), 1), ("inventory.1", potion("water"), 1),
         ("inventory.2", potion("water"), 1)]

NO_UPGRADE = (
    "cannot craft netherite_pickaxe: need 1 crafting_table, have 0: craft 1 from 4 any plank; need 4 any "
    "plank, have 0: craft 4 from 1 any log; need 1 any log, have 0; need 1 furnace, have 0: craft 1 from "
    "8 cobblestone/blackstone/cobbled_deepslate; need 8 cobblestone/blackstone/cobbled_deepslate, have 0; "
    "need 1 netherite_upgrade_smithing_template, have 0: craft 2 from 7 diamond and 1 "
    "netherite_upgrade_smithing_template and 1 netherrack; need 7 diamond, have 0; need 1 "
    "netherite_upgrade_smithing_template, have 0; need 1 netherrack, have 0; need 1 netherite_ingot, have "
    "0: craft 1 from 4 netherite_scrap and 4 gold_ingot; need 4 netherite_scrap, have 0: smelt 4 from 4 "
    "ancient_debris; need 4 ancient_debris, have 0; need 4 gold_ingot, have 0: smelt 4 from 4 gold_ore; "
    "need 4 gold_ore, have 0; need fuel for 8 more smelts: 1 coal or 6 logs or planks. Get 1 any log, 8 "
    "cobblestone/blackstone/cobbled_deepslate, 7 diamond, 1 netherite_upgrade_smithing_template, 1 "
    "netherrack, 4 ancient_debris, 4 gold_ore, 1 coal.")
NO_POTION = (
    "cannot craft healing_potion: need 1 crafting_table, have 0: craft 1 from 4 any plank; need 4 any "
    "plank, have 0: craft 4 from 1 any log; need 1 any log, have 0; need 1 furnace, have 0: craft 1 from "
    "8 cobblestone/blackstone/cobbled_deepslate; need 8 cobblestone/blackstone/cobbled_deepslate, have 0; "
    "need 1 awkward_potion, have 0: brew 1 from 1 water_potion and 1 nether_wart; need 1 nether_wart, "
    "have 0; need 1 glistering_melon_slice, have 0: craft 1 from 8 gold_nugget and 1 melon_slice; need 8 "
    "gold_nugget, have 0: smelt 8 from 8 one of 12 kinds such as golden_pickaxe; need 8 one of 12 kinds "
    "such as golden_pickaxe, have 0; need 1 melon_slice, have 0; need fuel for 8 more smelts: 1 coal or 6 "
    "logs or planks. Get 1 any log, 8 cobblestone/blackstone/cobbled_deepslate, 1 nether_wart, 8 one of "
    "12 kinds such as golden_pickaxe, 1 melon_slice, 1 coal.")


def site():
    rc(f"forceload add {X0 - 16} {Z0 - 16} {X1 + 16} {Z1 + 16}",
       f"fill {X0} {Y - 3} {Z0} {X1} {Y} {Z1} stone")
    clear_site()


def clear_site():
    """Removes stations and anything else above the floor; returns the stations it removed."""
    x0, y0, z0, x1, y1, z1 = BOX
    found = {}
    for block in STATIONS:
        m = FILLED_RE.search(rc(f"fill {x0} {y0} {z0} {x1} {y1} {z1} air replace minecraft:{block}"))
        if m:
            found[block] = int(m.group(1))
    zm = (z0 + z1) // 2
    rc(f"fill {x0} {y0} {z0} {x1} {y1} {zm} air", f"fill {x0} {y0} {zm + 1} {x1} {y1} {z1} air",
       f"kill @e[type=item,x={CX},y={Y},z={CZ},distance=..40]")
    return found


def goal(spec, timeout):
    n = task(spec)
    kind, text, wakes = outcome(n, timeout, "craft")
    time.sleep(2)
    return kind, text, sum(1 for w in wakes if w[1] == "goal_failed")


def have(item):
    n = count_item(NAME, item)
    return n if isinstance(n, int) else -1


def netherite():
    prep(items=SMITH_KIT, comp_at=COMP_AT)
    kind, text, _ = goal("craft netherite_pickaxe 1", 150)
    got, left = have("netherite_pickaxe"), have("diamond_pickaxe")
    return dict(ok=kind == "goal_done" and got == 1 and left == 0, outcome=kind, text=text,
                netherite_pickaxe=got, diamond_pickaxe=left)


def brew_two():
    prep(items=BREW_KIT + WATER[:1], comp_at=COMP_AT)
    k1, t1, _ = goal("craft awkward_potion 1", 150)
    awkward = have(potion("awkward"))
    k2, t2, _ = goal("craft healing_potion 1", 150)
    healing, left = have(potion("healing")), have(potion("awkward"))
    ok = k1 == k2 == "goal_done" and awkward == 1 and healing == 1 and left == 0
    return dict(ok=ok, outcome=[k1, k2], text=[t1, t2], awkward_after_first=awkward, healing=healing,
                awkward_left=left, powder_left=have("blaze_powder"))


def brew_chain():
    prep(items=BREW_KIT + WATER, comp_at=COMP_AT)
    kind, text, _ = goal("craft healing_potion 3", 240)
    healing, water = have(potion("healing")), have(potion("water"))
    return dict(ok=kind == "goal_done" and healing == 3 and water == 0, outcome=kind, text=text,
                healing=healing, water_left=water)


def impossible(spec, kit, want, item):
    prep(items=kit, comp_at=COMP_AT)
    kind, text, fails = goal(spec, 60)
    got = have(item)
    return dict(ok=kind == "goal_failed" and text == want and fails == 1 and got == 0, outcome=kind,
                failures=fails, list_matches=text == want, text=None if text == want else text)


CASES = dict(
    netherite=netherite, brew_two=brew_two, brew_chain=brew_chain,
    no_upgrade=lambda: impossible("craft netherite_pickaxe 1", [("hotbar.1", "diamond_pickaxe", 1),
                                  ("hotbar.2", "smithing_table", 1)], NO_UPGRADE, "netherite_pickaxe"),
    no_potion=lambda: impossible("craft healing_potion 1", [("hotbar.3", "blaze_powder", 1),
                                 ("hotbar.4", "brewing_stand", 1)] + WATER[:1], NO_POTION, potion("healing")))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--trials", type=int, default=5)
    ap.add_argument("--binary", default=None)
    ap.add_argument("cases", nargs="*")
    a = ap.parse_args()
    site()
    start(binary=a.binary)
    try:
        for name in a.cases or list(CASES):
            wins = 0
            for i in range(a.trials):
                clear_site()
                r = CASES[name]()
                r["stations_placed"] = clear_site()
                wins += r["ok"]
                print(json.dumps(dict(case=name, trial=i + 1, **r)), flush=True)
            print(json.dumps(dict(summary=name, ok=wins, trials=a.trials)), flush=True)
    finally:
        clear_site()
        stop()


if __name__ == "__main__":
    sys.exit(main())
