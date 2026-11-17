"""Ghost blocks: does the body come to believe in a block the server refused?

Usage: python3 blocks/ghost.py <binary|-> <trials> [case ...]

Each trial forces the server to refuse placements and then compares, cell by cell, what the
client's world holds (`probe` lines in the bot log) with what the server holds (RCON). A
ghost is a cell the client shows as a block and the server as air. The body's other record
of blocks, the ledger of placed cells, is checked the same way.

Cases:
  stand   a `place dirt` goal into a cell an armour stand occupies: every click is refused
          for an entity in the way, the goal gives up after its attempts
  pillar  the body shut in a one-cell bedrock well 4 deep with an armour stand in it and a
          go_to outside: the only way is a pillar, and the server refuses the blocks the
          armour stand overlaps
  stray   the 5x5 hut built by five fill goals (a scaffold pillar ends up under the roof
          and is dug from on top): counts the ticks the hand digs at a cell inside the
          room that the client already shows as air
"""
import json, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from kit import *

WELL = (CX + 6, CZ + 6)


def probe(cells):
    """What the client's world holds at each cell."""
    n = len(log_lines())
    for x, y, z in cells:
        send(f"probe {x} {y} {z}")
    want, out, end = set(cells), {}, time.time() + 5
    while time.time() < end and want - set(out):
        for l in log_lines()[n:]:
            p = l.split()
            if l.startswith("B ") and len(p) == 6:
                out[(int(p[2]), int(p[3]), int(p[4]))] = p[5]
        time.sleep(0.1)
    return out


def ghosts(cells):
    """Cells the client shows as a block and the server as air."""
    seen = probe(cells)
    return [(c, seen.get(c)) for c in cells
            if seen.get(c) not in (None, "air") and is_block(*c, "air")]


def clicks(since, cell):
    """Place clicks the hand sent at `cell` since log line `since`."""
    key = "hand=place:%d,%d,%d" % cell
    return sum(1 for l in log_lines()[since:] if l.startswith("T ") and key in l)


def ledger_cells():
    try:
        return {tuple(int(v) for v in l.split()[:3]) for l in open(f"{S}/bot.placed")
                if l[:1].isdigit() or l[:1] == "-"}
    except FileNotFoundError:
        return set()


def around(c):
    x, y, z = c
    return [c, (x + 1, y, z), (x - 1, y, z), (x, y + 1, z), (x, y - 1, z), (x, y, z + 1), (x, y, z - 1)]


def stand():
    cell = (CX + 3, Y + 1, CZ)
    prep(items=[("hotbar.1", "dirt", 16)])
    rc(f"summon armor_stand {cell[0] + 0.5} {cell[1]} {cell[2] + 0.5} {{NoGravity:1b,Invulnerable:1b}}")
    time.sleep(0.5)
    n = task("place dirt %d %d %d" % cell)
    kind, text, _ = outcome(n, 60, "place")
    time.sleep(1.0)
    g = ghosts(around(cell))
    return dict(ok=not g, outcome=kind, text=text, clicks=clicks(n, cell), server_air=is_block(*cell, "air"),
                ghosts=g, in_ledger=cell in ledger_cells())


def pillar():
    x, z = WELL
    rc(f"fill {x - 1} {Y + 1} {z - 1} {x + 1} {Y + 4} {z + 1} bedrock",
       f"fill {x} {Y + 1} {z} {x} {Y + 4} {z} air", f"setblock {x} {Y} {z} bedrock")
    prep(items=[("hotbar.1", "dirt", 16)], at=(x + 0.5, Y + 1, z + 0.5))
    rc(f"summon armor_stand {x + 0.5} {Y + 1} {z + 0.5} {{NoGravity:1b,Invulnerable:1b}}")
    time.sleep(0.5)
    n = task(f"go_to {x + 4.5} {Y + 1} {z + 0.5}")
    kind, text, _ = outcome(n, 90, "go_to")
    time.sleep(1.0)
    column = [(x, Y + dy, z) for dy in range(1, 6)]
    g = ghosts(column)
    sent = sum(clicks(n, c) for c in column)
    held = sum(not is_block(*c, "air") for c in column)
    out = dict(ok=not g, outcome=kind, text=text, clicks=sent, server_blocks_in_well=held, ghosts=g,
               ledger_ghosts=sorted(c for c in ledger_cells() if c in column and is_block(*c, "air")))
    rc(f"fill {x - 1} {Y + 1} {z - 1} {x + 1} {Y + 6} {z + 1} air", f"setblock {x} {Y} {z} stone")
    return out


def stray():
    """The hut of protect_hut.py built by the body: four walls and the roof by five fill
    goals. Laying the roof it stands on scaffold pillars inside the walls and digs them
    from on top before the roof closes. Dig ticks at a cell inside the room are split by
    whether the watch on that cell had already shown it as air."""
    x0, x1, z0, z1 = CX - 2, CX + 2, CZ - 2, CZ + 2
    rc(f"fill {x0 - 3} {Y + 1} {z0 - 3} {x1 + 3} {Y + 8} {z1 + 3} air",
       f"fill {X0} {Y - 3} {Z0} {X1} {Y} {Z1} stone replace air")
    prep(items=[("hotbar.0", "iron_pickaxe", 1), ("hotbar.1", "dirt", 64), ("hotbar.2", "dirt", 64)],
         at=(CX + 6.5, Y + 1, CZ - 6.5))
    send("unwatch")
    inside = [(x, y, z) for x in range(x0 + 1, x1) for z in range(z0 + 1, z1) for y in (Y + 1, Y + 2)]
    for c in inside:
        send("watch %d %d %d" % c)
    time.sleep(0.5)
    n0, texts = len(log_lines()), []
    for spec in (f"place dirt {x0} {Y + 1} {z0} {x1} {Y + 2} {z0}", f"place dirt {x0} {Y + 1} {z1} {x1} {Y + 2} {z1}",
                 f"place dirt {x0} {Y + 1} {z0 + 1} {x0} {Y + 2} {z1 - 1}",
                 f"place dirt {x1} {Y + 1} {z0 + 1} {x1} {Y + 2} {z1 - 1}",
                 f"place dirt {x0} {Y + 3} {z0} {x1} {Y + 3} {z1}"):
        n = task(spec)
        kind, text, _ = outcome(n, 300, "place")
        texts.append(f"{kind}: {text}")
    time.sleep(1.0)
    state, dig_on_block, dig_on_air = {}, 0, 0
    for l in log_lines()[n0:]:
        p = l.split()
        if l.startswith("B ") and len(p) == 6:
            state[(int(p[2]), int(p[3]), int(p[4]))] = p[5]
        elif l.startswith("T ") and "hand=dig:" in l:
            c = tuple(int(v) for v in l.rsplit("hand=dig:", 1)[1].split()[0].split(","))
            if c in inside:
                if state.get(c, "air") == "air":
                    dig_on_air += 1
                else:
                    dig_on_block += 1
    left = sum(not is_block(*c, "air") for c in inside)
    roof = sum(is_block(x, Y + 3, z, "dirt") for x in range(x0, x1 + 1) for z in range(z0, z1 + 1))
    send("unwatch")
    rc(f"fill {x0 - 3} {Y + 1} {z0 - 3} {x1 + 3} {Y + 8} {z1 + 3} air")
    return dict(ok=texts[-1].startswith("goal_done") and dig_on_air == 0 and left == 0 and roof == 25, roof=texts[-1],
                roof_cells=roof, dig_ticks_on_block=dig_on_block, dig_ticks_on_air=dig_on_air,
                scaffold_left_inside=left)


CASES = dict(stand=stand, pillar=pillar, stray=stray)

if __name__ == "__main__":
    binary = None if sys.argv[1] == "-" else sys.argv[1]
    trials = int(sys.argv[2])
    names = sys.argv[3:] or list(CASES)
    build_site()
    start(binary, companion=False)
    try:
        for name in names:
            rs = []
            for i in range(trials):
                r = CASES[name]()
                rs.append(r)
                print(json.dumps(dict(case=name, trial=i + 1, **r), default=str), flush=True)
            print(f"SUMMARY {name}: ok {sum(r['ok'] for r in rs)}/{len(rs)}, "
                  f"ghosts {sum(len(r.get('ghosts', [])) for r in rs)}, "
                  f"clicks {sum(r.get('clicks', 0) for r in rs)}, "
                  f"dig ticks on air {sum(r.get('dig_ticks_on_air', 0) for r in rs)}", flush=True)
    finally:
        rc(f"kill @e[type=!player,x={CX},y={Y},z={CZ},distance=..60]",
           f"fill {X0} {Y + 1} {Z0} {X1} {Y + 8} {Z0 + 20} air", f"fill {X0} {Y + 1} {Z0 + 21} {X1} {Y + 8} {Z1} air",
           f"fill {X0} {Y - 3} {Z0} {X1} {Y} {Z1} stone")
        stop()
