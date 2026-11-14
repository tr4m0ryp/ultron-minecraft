"""Gather goals live: mine stone with a pickaxe, mine a block the held tool cannot harvest
(expects a clean "missing tool" failure), collect dropped items. Usage: gather.py [case ...]"""
import json, sys
from kit import *

NEAR = (CX + 0.5, Y + 1, CZ + 0.5)


def mine_stone():
    prep(items=[("hotbar.0", "wooden_pickaxe", 1)], comp_at=(CX + 3.5, Y + 1, CZ + 3.5))
    # A 3-block stone pillar next to the bot; the floor is stone too.
    rc(f"fill {CX + 2} {Y + 1} {CZ} {CX + 2} {Y + 3} {CZ} stone")
    n = task("mine stone 3")
    kind, text, _ = outcome(n, 60, "mine")
    time.sleep(2)
    return dict(case="mine_stone", outcome=kind, text=text,
                cobblestone=count_item("Ultron", "cobblestone"))


def mine_missing_tool():
    prep(items=[("hotbar.0", "wooden_pickaxe", 1)], comp_at=(CX + 3.5, Y + 1, CZ + 3.5))
    rc(f"setblock {CX + 2} {Y + 1} {CZ} iron_ore", f"setblock {CX - 2} {Y + 1} {CZ} obsidian")
    out = {}
    for block in ("iron_ore", "obsidian"):
        n = task(f"mine {block} 1")
        kind, text, _ = outcome(n, 30, "mine")
        out[block] = dict(outcome=kind, text=text)
    still = dict(iron_ore=is_block(CX + 2, Y + 1, CZ, "iron_ore"),
                 obsidian=is_block(CX - 2, Y + 1, CZ, "obsidian"))
    return dict(case="mine_missing_tool", **out, blocks_intact=still)


def collect():
    prep(comp_at=(CX + 3.5, Y + 1, CZ + 3.5))
    rc(f'summon item {CX + 6.5} {Y + 1} {CZ + 0.5} {{Item:{{id:"minecraft:oak_log",count:2}}}}',
       f'summon item {CX - 5.5} {Y + 1} {CZ + 4.5} {{Item:{{id:"minecraft:oak_log",count:1}}}}',
       f'summon item {CX + 0.5} {Y + 1} {CZ - 7.5} {{Item:{{id:"minecraft:dirt",count:4}}}}')
    n = task("collect oak_log 3")
    kind, text, _ = outcome(n, 60, "collect")
    time.sleep(1)
    return dict(case="collect", outcome=kind, text=text,
                oak_log=count_item("Ultron", "oak_log"), dirt=count_item("Ultron", "dirt"))


CASES = dict(mine_stone=mine_stone, mine_missing_tool=mine_missing_tool, collect=collect)

if __name__ == "__main__":
    names = sys.argv[1:] or list(CASES)
    build_site()
    start()
    try:
        for name in names:
            print(json.dumps(CASES[name]()), flush=True)
    finally:
        stop()
