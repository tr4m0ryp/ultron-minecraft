"""Mining yield and roof building, live.

Usage: python3 building.py <binary|-> <trials> [mine|roof ...]
  mine  the goal site turned into ground: a dirt top at y 100 over stone down to y 86; the
        bot on the surface with an iron pickaxe runs `mine stone 16`. Stone broken comes
        from the server's mined statistic, picked up from the cobblestone in the
        inventory.
  roof  a 5x5 room of oak planks, walls 2 high on the floor at y 100 with a doorway, the
        bot inside with 30 planks and 16 dirt runs one fill goal for the 5x5 roof at
        y 103. Cells placed out of 25, scaffold (dirt) left standing afterwards.
The site is restored to its plain floor (open air below) at the end. One JSON line per
trial.
"""
import json, signal, sys
from kit import *

TOP = Y + 16
RX, RZ = 1418, 1018


def clear():
    rc(f"fill {X0} {Y + 1} {Z0} {X1} {TOP} {Z1} air",
       f"fill {X0} {Y - 14} {Z0} {X1} {Y} {Z1} stone",
       f"kill @e[type=item,x={CX},y={Y},z={CZ},distance=..80]")


def stat(objective):
    out = rc(f"scoreboard players get Ultron {objective}")
    m = re.search(r"has (\d+)", out)
    return int(m.group(1)) if m else 0


def mine():
    rc(f"fill {X0} {Y} {Z0} {X1} {Y} {Z1} dirt",
       "scoreboard objectives add mined_stone minecraft.mined:minecraft.stone",
       "scoreboard players set Ultron mined_stone 0")
    prep(items=[("hotbar.0", "iron_pickaxe", 1)], at=(CX + 0.5, Y + 1, CZ + 0.5))
    t0 = time.time()
    n = task("mine stone 16")
    kind, text, _ = outcome(n, 300, "mine")
    time.sleep(2)
    broken = stat("mined_stone")
    got = count_item("Ultron", "cobblestone")
    got = got if isinstance(got, int) else 0
    return dict(ok=kind == "goal_done", outcome=kind, secs=round(time.time() - t0, 1),
                broken=broken, picked=got, ratio=round(got / broken, 3) if broken else None,
                text=text)


def roof():
    x0, z0, x1, z1 = RX, RZ, RX + 4, RZ + 4
    rc(f"fill {x0} {Y + 1} {z0} {x1} {Y + 2} {z1} oak_planks",
       f"fill {x0 + 1} {Y + 1} {z0 + 1} {x1 - 1} {Y + 2} {z1 - 1} air",
       f"fill {x0 + 2} {Y + 1} {z0} {x0 + 2} {Y + 2} {z0} air")
    prep(items=[("hotbar.0", "oak_planks", 30), ("hotbar.1", "dirt", 16),
                ("hotbar.2", "iron_shovel", 1)], at=(x0 + 2.5, Y + 1, z0 + 2.5))
    t0 = time.time()
    n = task(f"place oak_planks {x0} {Y + 3} {z0} {x1} {Y + 3} {z1}")
    kind, text, _ = outcome(n, 400, "place")
    time.sleep(2)
    cells = sum(is_block(x, Y + 3, z, "oak_planks")
                for x in range(x0, x1 + 1) for z in range(z0, z1 + 1))
    scaffold = sum(is_block(x, y, z, "dirt") for x in range(x0 - 3, x1 + 4)
                   for z in range(z0 - 3, z1 + 4) for y in range(Y + 1, Y + 7))
    return dict(ok=cells == 25, outcome=kind, secs=round(time.time() - t0, 1), cells=cells,
                scaffold_left=scaffold, text=text)


CASES = dict(mine=mine, roof=roof)

if __name__ == "__main__":
    # A batch started in the background ignores SIGINT; TERM still restores the site.
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(1))
    binary, trials = sys.argv[1], int(sys.argv[2])
    names = sys.argv[3:] or list(CASES)
    build_site()
    rc("gamerule spawn_mobs false", "gamerule advance_time false", "time set noon")
    start(None if binary == "-" else binary, companion=False)
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
        # Under the site's 4-thick floor was open air: put it back.
        rc(f"fill {X0} {Y - 14} {Z0} {X1} {Y - 4} {Z1} air",
           "scoreboard objectives remove mined_stone")
        build_site()
        rc("gamerule advance_time true", "gamerule spawn_mobs true")
