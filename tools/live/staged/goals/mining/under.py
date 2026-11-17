"""Ore reached from below: what the mine goal reports when its route digs the ore itself.

Usage: python3 mining/under.py <binary|-> <trials>

Two iron ore blocks sit on the floor inside bedrock boxes that are open underneath (stone
floor below them), nearest the body; two more stand in the open farther off. `mine
iron_ore 2`: the only way to a boxed ore is a tunnel under the box and up, and the route
planner breaks the ore as part of that tunnel. Read on the server: which ores are gone and
the raw iron held. Read from the goal: its "mined N iron_ore, picked up M raw_iron".
"""
import json, re, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from kit import *

BOXED = [(X0 + 8, Y + 1, Z0 + 14), (X0 + 8, Y + 1, Z0 + 26)]
OPEN = [(X0 + 30, Y + 1, Z0 + 16), (X0 + 30, Y + 1, Z0 + 24)]


def trial():
    rc(f"fill {X0} {Y - 3} {Z0} {X1} {Y} {Z1} stone", f"fill {X0} {Y + 1} {Z0} {X1} {Y + 6} {Z0 + 20} air",
       f"fill {X0} {Y + 1} {Z0 + 21} {X1} {Y + 6} {Z1} air")
    for x, y, z in BOXED:
        rc(f"fill {x - 1} {y} {z - 1} {x + 1} {y + 2} {z + 1} bedrock", f"setblock {x} {y} {z} iron_ore")
    rc(*[f"setblock {x} {y} {z} iron_ore" for x, y, z in OPEN])
    prep(items=[("hotbar.0", "stone_pickaxe", 1)], at=(X0 + 8.5, Y + 1, Z0 + 20.5))
    n = task("mine iron_ore 2")
    kind, text, _ = outcome(n, 240, "mine")
    time.sleep(1.5)
    boxed_gone = sum(not is_block(*c, "iron_ore") for c in BOXED)
    open_gone = sum(not is_block(*c, "iron_ore") for c in OPEN)
    held = count_item(NAME, "raw_iron")
    m = re.search(r"mined (\d+) iron_ore, picked up (\d+) raw_iron", text or "")
    said = (int(m.group(1)), int(m.group(2))) if m else None
    gone = boxed_gone + open_gone
    return dict(ok=kind == "goal_done" and said == (gone, held), outcome=kind, text=text, said_mined=said and said[0],
                said_picked_up=said and said[1], server_ores_gone=gone, boxed_gone=boxed_gone, open_gone=open_gone,
                server_raw_iron=held)


if __name__ == "__main__":
    binary = None if sys.argv[1] == "-" else sys.argv[1]
    build_site()
    start(binary, companion=False)
    rs = []
    try:
        for i in range(int(sys.argv[2])):
            r = trial()
            rs.append(r)
            print(json.dumps(dict(trial=i + 1, **r)), flush=True)
    finally:
        rc(f"fill {X0} {Y - 3} {Z0} {X1} {Y} {Z1} stone", f"fill {X0} {Y + 1} {Z0} {X1} {Y + 6} {Z0 + 20} air",
           f"fill {X0} {Y + 1} {Z0 + 21} {X1} {Y + 6} {Z1} air",
           f"kill @e[type=item,x={CX},y={Y},z={CZ},distance=..60]")
        stop()
    print(f"SUMMARY report equals server {sum(r['ok'] for r in rs)}/{len(rs)}; said mined "
          f"{[r['said_mined'] for r in rs]}, server ores gone {[r['server_ores_gone'] for r in rs]}, "
          f"raw iron {[r['server_raw_iron'] for r in rs]}")
