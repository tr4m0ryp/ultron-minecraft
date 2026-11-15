"""Full goal chains live, set step by step the way thought would set them.

bed:  kit is a sword or shears plus 3 oak_log; 3 white sheep 10 to 20 blocks away; the
      player's own 5x5 plank shelter (walls 2 high, roof, a 1-wide door) built by RCON.
      collect white_wool 3 -> craft white_bed 1 -> place white_bed <cell inside> ->
      night -> sleep_in_bed. Verified over RCON: wool count, bed foot and head inside the
      shelter, sleeping_pos in the player data, every shelter block still standing.
food: kit is 8 cobblestone, 2 oak_log, 1 coal and an iron sword; food driven low with the
      hunger effect; 3 cows nearby. collect beef 3 -> smelt cooked_beef 3 -> eat_now.
      Verified over RCON: beef and cooked_beef counts, foodLevel before and after.

bed_hill: the bed chain with the shelter dug into a hill, reached by a corridor: the body
      must stand inside to place the bed, so the bed's facing decides whether it fits.

Usage: chain.py [-n trials] [--bin path] [case ...]   cases: bed_sword bed_shears bed_hill food
Prints one JSON line per trial (per-step outcome and seconds) and a summary line.
"""
import argparse, json, re, sys
from kit import *

COMP_AT = (CX - 5.5, Y + 1, CZ - 8.5)
BOX = (X0, Y + 1, Z0, X1, Y + 12, Z1)
FILLED_RE = re.compile(r"filled (\d+) block")
# Shelter: 5x5 centred on (HX, HZ), walls y+1..y+2, roof y+3, door gap in the west wall.
HX, HZ = CX + 8, CZ - 6
WALL = "oak_planks"
DOOR = [(HX - 2, Y + 1, HZ), (HX - 2, Y + 2, HZ)]
SHELL = [(x, y, z) for y in (Y + 1, Y + 2) for x in range(HX - 2, HX + 3)
         for z in range(HZ - 2, HZ + 3) if (x in (HX - 2, HX + 2) or z in (HZ - 2, HZ + 2))
         and (x, y, z) not in DOOR] + \
        [(x, Y + 3, z) for x in range(HX - 2, HX + 3) for z in range(HZ - 2, HZ + 3)]
INSIDE = [(x, Y + 1, z) for x in range(HX - 1, HX + 2) for z in range(HZ - 1, HZ + 2)]
# The bed against the back (east) wall, opposite the door: a bot that stands in the middle
# and faces it would put the head into the wall.
BED = (HX + 1, Y + 1, HZ)


def site():
    rc(f"forceload add {X0 - 16} {Z0 - 16} {X1 + 16} {Z1 + 16}",
       f"fill {X0} {Y - 3} {Z0} {X1} {Y} {Z1} stone")
    clear_site()


def clear_site():
    """Everything above the floor goes; returns the beds, stations and shelter blocks found."""
    x0, y0, z0, x1, y1, z1 = BOX
    found = {}
    for b in ("crafting_table", "furnace", "white_bed", WALL):
        out = rc(f"fill {x0} {y0} {z0} {x1} {y1} {z1} air replace minecraft:{b}")
        m = FILLED_RE.search(out)
        found[b] = int(m.group(1)) if m else 0
    zm = (z0 + z1) // 2
    sel = f"x={CX},y={Y},z={CZ},distance=..60"
    rc(f"fill {x0} {y0} {z0} {x1} {y1} {zm} air", f"fill {x0} {y0} {zm + 1} {x1} {y1} {z1} air",
       f"kill @e[type=!player,{sel}]")
    time.sleep(1.5)
    rc(f"kill @e[type=item,{sel}]")
    return found


def shelter():
    rc(f"fill {HX - 2} {Y + 1} {HZ - 2} {HX + 2} {Y + 3} {HZ + 2} {WALL}",
       f"fill {HX - 1} {Y + 1} {HZ - 1} {HX + 1} {Y + 2} {HZ + 1} air",
       *[f"setblock {x} {y} {z} air" for x, y, z in DOOR])


def broken():
    """Shelter cells without their plank, each miss read twice (an RCON hiccup is no hole)."""
    miss = [c for c in SHELL if not is_block(*c, WALL)]
    return [c for c in miss if not is_block(*c, WALL)]


def summon(kind, spots):
    for dx, dz in spots:
        rc(f"summon {kind} {CX + dx + 0.5} {Y + 1} {CZ + dz + 0.5} "
           f"{{PersistenceRequired:1b,Age:0,Color:0b}}")
    time.sleep(1.0)


def step(spec, timeout):
    t0 = time.time()
    n = task(spec)
    kind, text, _ = outcome(n, timeout)
    secs = round(time.time() - t0, 1)
    time.sleep(1.0)
    return dict(spec=spec, outcome=kind, text=text, secs=secs, ok=kind == "goal_done")


def nbt_int(path):
    m = re.search(r"data: (-?\d+)", rc(f"data get entity {NAME} {path}"))
    return int(m.group(1)) if m else None


def num(v):
    return v if isinstance(v, int) else -1


def bed_cells():
    """Foot and head of a white bed inside the shelter, server-side."""
    foot = [c for c in INSIDE if "passed" in rc(
        f"execute if block {c[0]} {c[1]} {c[2]} minecraft:white_bed[part=foot]").lower()]
    head = [c for c in INSIDE if "passed" in rc(
        f"execute if block {c[0]} {c[1]} {c[2]} minecraft:white_bed[part=head]").lower()]
    return foot, head


def hill():
    """The shelter dug into a hill: stone 3 blocks around it, 4 high, and a corridor to the
    door. No spot outside the walls reaches the bed, so the body stands inside, where a bed
    against the back wall only fits facing north or south."""
    rc(f"fill {HX - 5} {Y + 1} {HZ - 5} {HX + 5} {Y + 4} {HZ + 5} stone")
    shelter()
    rc(f"fill {HX - 5} {Y + 1} {HZ} {HX - 3} {Y + 2} {HZ} air")


def bed(tool, dug_in=False):
    clear_site()
    hill() if dug_in else shelter()
    prep(items=[("hotbar.0", tool, 1), ("hotbar.1", "oak_log", 3)], comp_at=COMP_AT)
    rc(f"kill @e[type=item,x={CX},y={Y},z={CZ},distance=..60]")
    summon("sheep", [(-12, 6), (-14, 10), (-10, 14)])
    steps = [step("collect white_wool 3", 120)]
    wool = num(count_item(NAME, "white_wool"))
    steps.append(step("craft white_bed 1", 120))
    beds = num(count_item(NAME, "white_bed"))
    steps.append(step(f"place white_bed {BED[0]} {BED[1]} {BED[2]}", 120))
    foot, head = bed_cells()
    rc("time set midnight")
    time.sleep(1.0)
    steps.append(step("sleep_in_bed", 60))
    # The path alone: the whole player record is split over several RCON packets.
    slept_at = rc(f"data get entity {NAME} sleeping_pos")
    sleeping = "has the following" in slept_at
    # Wake: a teleport takes the player out of bed; then the clock goes back to noon.
    rc(f"tp {NAME} {CX + 0.5} {Y + 1} {CZ + 0.5}", "time set noon")
    gone = broken()
    sheep = rc(f"execute if entity @e[type=sheep,x={CX},y={Y},z={CZ},distance=..60]")
    m = re.search(r"Count: (\d+)", sheep)
    ok = (wool >= 3 and beds >= 1 and foot == [BED] and len(head) == 1 and sleeping
          and not gone)
    case = f"bed_{'shears' if tool == 'shears' else 'sword'}{'_hill' if dug_in else ''}"
    return dict(case=case, ok=ok, wool=wool,
                beds_crafted=beds, bed_foot=foot, bed_head=head, sleeping=sleeping,
                sleeping_pos=slept_at.split(": ", 1)[-1], shelter_broken=gone, sheep_alive=int(m.group(1)) if m else 0,
                steps=steps, total_secs=round(sum(s["secs"] for s in steps), 1))


def food():
    clear_site()
    prep(items=[("hotbar.0", "iron_sword", 1), ("hotbar.1", "cobblestone", 8),
                ("hotbar.2", "oak_log", 2), ("hotbar.3", "coal", 1)], comp_at=COMP_AT)
    rc(f"kill @e[type=item,x={CX},y={Y},z={CZ},distance=..60]",
       f"effect clear {NAME} saturation", f"effect give {NAME} hunger 60 255")
    for _ in range(60):
        time.sleep(0.5)
        if num(nbt_int("foodLevel")) <= 8:
            break
    rc(f"effect clear {NAME} hunger")
    low = nbt_int("foodLevel")
    summon("cow", [(7, 5), (9, 5), (11, 5)])
    steps = [step("collect beef 3", 120)]
    beef = num(count_item(NAME, "beef"))
    steps.append(step("smelt cooked_beef 3", 180))
    cooked = num(count_item(NAME, "cooked_beef"))
    steps.append(step("eat_now", 30))
    after = nbt_int("foodLevel")
    left = num(count_item(NAME, "cooked_beef"))
    stations = clear_site()
    ok = (beef >= 3 and cooked >= 3 and num(after) > num(low)
          and steps[-1]["ok"])
    return dict(case="food", ok=ok, food_low=low, food_after=after, beef=beef,
                cooked=cooked, cooked_left=left, furnaces=stations["furnace"],
                tables=stations["crafting_table"], steps=steps,
                total_secs=round(sum(s["secs"] for s in steps), 1))


CASES = dict(bed_sword=lambda: bed("iron_sword"), bed_shears=lambda: bed("shears"),
             bed_hill=lambda: bed("shears", dug_in=True), food=food)

if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("-n", type=int, default=5)
    ap.add_argument("--bin")
    ap.add_argument("cases", nargs="*")
    a = ap.parse_args()
    for f in (f"{S}/bot.placed",):
        Path(f).unlink(missing_ok=True)
    site()
    start(binary=a.bin)
    tally = {}
    try:
        for name in a.cases or list(CASES):
            for t in range(a.n):
                r = CASES[name]()
                r["trial"] = t
                tally.setdefault(name, []).append(bool(r["ok"]))
                print(json.dumps(r), flush=True)
    finally:
        rc("time set noon")
        clear_site()
        stop()
    print(json.dumps({k: f"{sum(v)}/{len(v)}" for k, v in tally.items()}), flush=True)
