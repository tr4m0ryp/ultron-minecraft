"""Item goals live: craft (2x2 grid and crafting table, the wooden pickaxe chain from logs),
give (toss to the companion, checked in its inventory), place. Usage: items.py [case ...]"""
import json, sys
from kit import *

TABLE = (CX + 2, Y + 1, CZ + 2)


def run(spec, label, timeout=40):
    n = task(spec)
    kind, text, _ = outcome(n, timeout, label)
    return kind, text


def craft_chain():
    """3 logs -> 12 planks -> 4 sticks (2x2), pickaxe without a table fails, then with one."""
    prep(items=[("hotbar.1", "oak_log", 3)], comp_at=(CX - 3.5, Y + 1, CZ - 3.5))
    rc(f"fill {TABLE[0] - 4} {TABLE[1]} {TABLE[2] - 4} {TABLE[0] + 4} {TABLE[1] + 2} {TABLE[2] + 4} air")
    steps = []
    for spec in ("craft oak_planks 12", "craft stick 4", "craft wooden_pickaxe 1"):
        kind, text = run(spec, "craft")
        steps.append(dict(spec=spec, outcome=kind, text=text))
    rc(f"setblock {TABLE[0]} {TABLE[1]} {TABLE[2]} crafting_table")
    time.sleep(1.5)
    kind, text = run("craft wooden_pickaxe 1", "craft", 60)
    steps.append(dict(spec="craft wooden_pickaxe 1 (table placed)", outcome=kind, text=text))
    time.sleep(1)
    inv = {i: count_item("Ultron", i) for i in ("oak_log", "oak_planks", "stick", "wooden_pickaxe")}
    return dict(case="craft_chain", steps=steps, server_inventory=inv)


def give():
    prep(items=[("hotbar.1", "oak_planks", 8)], comp_at=(CX + 8.5, Y + 1, CZ + 0.5))
    rc(f"gamemode survival {COMP}", f"clear {COMP}")
    kind, text = run("give companion oak_planks 3", "give", 40)
    time.sleep(3)
    return dict(case="give", outcome=kind, text=text,
                companion_planks=count_item(COMP, "oak_planks"),
                bot_planks=count_item("Ultron", "oak_planks"))


def place():
    prep(items=[("hotbar.1", "cobblestone", 4)], comp_at=(CX - 3.5, Y + 1, CZ - 3.5))
    spot = (CX + 5, Y + 1, CZ + 3)
    rc(f"setblock {spot[0]} {spot[1]} {spot[2]} air")
    kind, text = run(f"place cobblestone {spot[0]} {spot[1]} {spot[2]}", "place", 40)
    time.sleep(1)
    return dict(case="place", outcome=kind, text=text,
                placed=is_block(*spot, "cobblestone"), left=count_item("Ultron", "cobblestone"))


CASES = dict(craft_chain=craft_chain, give=give, place=place)

if __name__ == "__main__":
    names = sys.argv[1:] or list(CASES)
    build_site()
    start()
    try:
        for name in names:
            print(json.dumps(CASES[name]()), flush=True)
    finally:
        stop()
