"""Drops from animals live: `collect` hunts or shears when no drop lies on the ground.
Cases: kill_wool (sword, sheep killed), shear_wool (shears, the sheep must survive),
hunt_beef (collect beef 2 from cows), no_sheep (precise failure), sleep_shelter (bed inside
a small closed hut, by day: walk, aim, click; the click sets the spawn point), hungry
(empty inventory: thought is woken with the way to food).
Every outcome is read back over RCON. Usage: drops.py [-n trials] [--bin path] [case ...]"""
import argparse, json, re, sys
from kit import *

COMP_AT = (CX - 5.5, Y + 1, CZ - 8.5)


def site():
    """kit.build_site without the server-wide gamerule (left as the server has it)."""
    rc(f"forceload add {X0 - 16} {Z0 - 16} {X1 + 16} {Z1 + 16}",
       f"fill {X0} {Y - 3} {Z0} {X1} {Y} {Z1} stone",
       f"fill {X0} {Y + 1} {Z0} {X1} {Y + 12} {Z0 + 20} air",
       f"fill {X0} {Y + 1} {Z0 + 21} {X1} {Y + 12} {Z1} air",
       f"kill @e[type=!player,x={CX},y={Y},z={CZ},distance=..60]")


def clean():
    """Animals killed by `kill` still drop their loot (prep's own clean-up does that), and a
    later trial would collect it for free: let the deaths happen, then clear the items."""
    sel = f"x={CX},y={Y},z={CZ},distance=..60"
    rc(f"kill @e[type=!player,{sel}]")
    time.sleep(1.5)
    rc(f"kill @e[type=item,{sel}]")
    time.sleep(0.5)


def setup(items=()):
    prep(items=items, comp_at=COMP_AT)
    clean()


def count(kind, extra=""):
    out = rc(f"execute if entity @e[type={kind},x={CX},y={Y},z={CZ},distance=..40{extra}]")
    m = re.search(r"Count: (\d+)", out)
    return int(m.group(1)) if m else 0


def summon(kind, dx, dz, n=1):
    for i in range(n):
        rc(f"summon {kind} {CX + dx + 0.5 + 2 * i} {Y + 1} {CZ + dz + 0.5} {{PersistenceRequired:1b,Age:0}}")
    time.sleep(1.0)


def goal(spec, label="collect", timeout=60):
    n = task(spec)
    kind, text, _ = outcome(n, timeout, label)
    time.sleep(1.0)
    return kind, text


def kill_wool():
    setup(items=[("hotbar.0", "stone_sword", 1)])
    summon("sheep", 8, 4)
    kind, text = goal("collect white_wool 1")
    wool = count_item(NAME, "white_wool")
    return dict(case="kill_wool", outcome=kind, text=text, wool=wool,
                ok=isinstance(wool, int) and wool >= 1)


def shear_wool():
    setup(items=[("hotbar.0", "stone_sword", 1), ("hotbar.1", "shears", 1)])
    summon("sheep", 8, 4)
    kind, text = goal("collect white_wool 1")
    wool = count_item(NAME, "white_wool")
    alive, shorn = count("sheep"), count("sheep", ",nbt={Sheared:1b}")
    return dict(case="shear_wool", outcome=kind, text=text, wool=wool, sheep_alive=alive,
                sheared=shorn, ok=isinstance(wool, int) and wool >= 1 and alive == 1)


def hunt_beef():
    setup(items=[("hotbar.0", "stone_sword", 1)])
    summon("cow", 7, 5, n=3)
    kind, text = goal("collect beef 2", timeout=90)
    beef = count_item(NAME, "beef")
    return dict(case="hunt_beef", outcome=kind, text=text, beef=beef, cows_left=count("cow"),
                ok=isinstance(beef, int) and beef >= 2)


def no_sheep():
    setup(items=[("hotbar.0", "stone_sword", 1)])
    summon("cow", 7, 5)
    kind, text = goal("collect white_wool 1", timeout=30)
    precise = kind == "goal_failed" and "no sheep within" in (text or "")
    return dict(case="no_sheep", outcome=kind, text=text, cows_left=count("cow"), ok=precise)


def sleep_shelter():
    """A 5x5 stone hut (3x3 inside, roof, door opening closed by a 1-block gap the planner
    must dig), a bed inside; the bot starts 8 blocks away. By day the click only sets the
    spawn point, which is the server-side proof that walk, aim and click worked."""
    setup()
    hx, hz = CX + 8, CZ - 6
    rc(f"fill {hx - 2} {Y + 1} {hz - 2} {hx + 2} {Y + 3} {hz + 2} stone",
       f"fill {hx - 1} {Y + 1} {hz - 1} {hx + 1} {Y + 2} {hz + 1} air",
       f"setblock {hx} {Y + 1} {hz} red_bed[facing=north,part=foot]",
       f"setblock {hx} {Y + 1} {hz - 1} red_bed[facing=north,part=head]",
       f"spawnpoint {NAME} {CX} {Y + 1} {CZ}")
    time.sleep(1.0)
    kind, text = goal("sleep_in_bed", "sleep_in_bed", timeout=60)
    slept = "sleeping_pos" in rc(f"data get entity {NAME}")
    spawn = rc(f"data get entity {NAME} respawn")
    # Out of bed at once (a teleport wakes the player): the clock is server-wide.
    rc(f"tp {NAME} {CX + 0.5} {Y + 1} {CZ + 0.5}")
    m = re.search(r"pos: \[I; (-?\d+), (-?\d+), (-?\d+)\]", spawn)
    at = tuple(int(v) for v in m.groups()) if m else None
    rc(f"fill {hx - 2} {Y + 1} {hz - 2} {hx + 2} {Y + 3} {hz + 2} air")
    ok = at is not None and abs(at[0] - hx) <= 0 and abs(at[2] - hz) <= 1
    return dict(case="sleep_shelter", outcome=kind, text=text, respawn=at, slept=slept,
                day=rc("time query day"), ok=ok)


def hungry():
    """Hungry with an empty inventory: thought hears `hungry_no_food` (a wake) at once."""
    setup()
    since = len(log_lines())
    rc(f"effect give {NAME} hunger 20 255")
    woke, food = None, None
    for _ in range(40):
        time.sleep(0.5)
        woke = next((l for l in log_lines()[since:] if "hungry_no_food" in l
                     and l.startswith("WAKE")), None)
        if woke:
            break
    rc(f"effect clear {NAME} hunger")
    out = rc(f"data get entity {NAME} foodLevel")
    m = re.search(r"data: (\d+)", out)
    food = int(m.group(1)) if m else out
    return dict(case="hungry", woke=woke, food=food, ok=woke is not None)


CASES = dict(kill_wool=kill_wool, shear_wool=shear_wool, hunt_beef=hunt_beef,
             no_sheep=no_sheep, sleep_shelter=sleep_shelter, hungry=hungry)

if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("-n", type=int, default=5)
    ap.add_argument("--bin")
    ap.add_argument("cases", nargs="*")
    a = ap.parse_args()
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
        clean()
        stop()
    print(json.dumps({k: f"{sum(v)}/{len(v)}" for k, v in tally.items()}), flush=True)
