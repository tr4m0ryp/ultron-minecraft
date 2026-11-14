"""Item 3: staged survival under real pressure, brain absent (no goals, no input).

Usage: python3 survival.py <label> [binary] [only-ids]
Survival mode, frozen midnight, iron sword + 8 bread, a 30x30 arena with 5-high walls at
y=100 (x 1200..1229, z 1100..1129). Four waves of 225 s (15 min): zombies, skeletons,
creepers, then a mix; each wave summons named mobs 9 to 13 blocks from the bot, tops up at
90 s, and kills the leftovers when it ends. On death the bot is brought back with a fresh
kit. Per wave: damage taken, deaths with cause, kills, reflex onsets and legs ownership.
"""
import json, math, random, re, sys, time
from arena import *

X0, Z0, N, Y = 1200, 1100, 30, 100
CX, CZ = X0 + N / 2, Z0 + N / 2
WAVE_S, TOPUP_S = 225, 90
WAVES = [
    ("zombies", ["zombie"] * 4, ["zombie"] * 2),
    ("skeletons", ["skeleton"] * 3, ["skeleton"] * 1),
    ("creepers", ["creeper"] * 3, ["creeper"] * 2),
    ("mix", ["zombie", "zombie", "skeleton", "creeper"], ["zombie", "skeleton", "creeper"]),
]
E_RE = re.compile(r"^E (\d+) (\w+) (\S+) (\S*) ?(\S+)")
KIT = (("hotbar.0", "iron_sword", 1), ("hotbar.7", "bread", 8))


def build():
    x1, z1 = X0 + N - 1, Z0 + N - 1
    rc("forceload add 1184 1084 1246 1146",
       f"fill {X0 - 1} {Y - 4} {Z0 - 1} {x1 + 1} {Y - 1} {z1 + 1} stone",
       f"fill {X0 - 1} {Y} {Z0 - 1} {x1 + 1} {Y + 4} {z1 + 1} stone",
       f"fill {X0} {Y} {Z0} {x1} {Y + 12} {z1} air")


def kit():
    prep_bot(night=True, items=KIT)


def bot_pos():
    rs = rows(since_ms=now_ms() - 1500)
    return (rs[-1]["x"], rs[-1]["z"]) if rs else (CX, CZ)


def summon(kinds, wave, start, rng):
    bx, bz = bot_pos()
    for k, kind in enumerate(kinds):
        for _ in range(20):
            a, d = rng.uniform(0, 2 * math.pi), rng.uniform(9, 13)
            x, z = bx + d * math.cos(a), bz + d * math.sin(a)
            if X0 + 1 < x < X0 + N - 1 and Z0 + 1 < z < Z0 + N - 1:
                break
        name = f"w{wave}{kind[0]}{start + k}"
        extra = ',equipment:{mainhand:{id:"minecraft:bow",count:1}}' if kind == "skeleton" else ""
        rc(f'summon {kind} {x:.1f} {Y} {z:.1f} {{CustomName:"{name}",PersistenceRequired:1b{extra}}}')


def watch(seconds, L0, deaths_seen):
    end = time.time() + seconds
    while time.time() < end:
        time.sleep(1.0)
        d = deaths(L0)
        if len(d) > deaths_seen[0]:
            deaths_seen[0] = len(d)
            time.sleep(2.5)
            rc(f"tp Ultron {CX} {Y} {CZ}")
            kit()


def wave_stats(name, t0, L0, rs):
    dmg = sum(max(0.0, a["hp"] - b["hp"]) for a, b in zip(rs, rs[1:]) if b["hp"] > 0 or a["hp"] > 0)
    nd = named_deaths(L0)
    kills = [n for n, why in nd if "Ultron" in why]
    fired, legs = {}, {}
    if rs:
        for e in events(since_tick=rs[0]["tick"]):
            m = E_RE.match(e)
            if m and int(m.group(1)) <= rs[-1]["tick"] and re.match(r"^[A-Z]+\d+$", m.group(5)):
                key = f"{m.group(5)}:{m.group(3)}"
                fired[key] = fired.get(key, 0) + 1
        for r in rs:
            legs[r["legs"]] = legs.get(r["legs"], 0) + 1
    return dict(wave=name, damage=round(dmg, 1), deaths=deaths(L0), kills=len(kills),
                mob_deaths=[f"{n}: {why}" for n, why in nd], fired=fired, legs_ticks=legs,
                min_hp=min((r["hp"] for r in rs), default=None),
                food_end=rs[-1]["food"] if rs else None)


def run(label, binary=None, only=None, seed=7):
    rng = random.Random(seed)
    build()
    rc("gamerule spawn_mobs false", "gamerule advance_time false", "gamerule mob_griefing false",
       "difficulty normal", f"kill @e[type=!player,x={CX},y={Y},z={CZ},distance=..40]")
    bot_start(binary, only)
    out = []
    try:
        rc(f"tp Ultron {CX} {Y} {CZ}")
        kit()
        time.sleep(2)
        for w, (name, first_batch, topup) in enumerate(WAVES, 1):
            L0, t0, seen = log_len(), now_ms(), [0]
            summon(first_batch, w, 1, rng)
            watch(TOPUP_S, L0, seen)
            summon(topup, w, len(first_batch) + 1, rng)
            watch(WAVE_S - TOPUP_S, L0, seen)
            rc(f"kill @e[type=!player,type=!item,x={CX},y={Y},z={CZ},distance=..40]")
            time.sleep(1)
            st = wave_stats(name, t0, L0, rows(since_ms=t0))
            out.append(st)
            print(label, json.dumps(st), flush=True)
    finally:
        rc("kill @e[type=item,x=1215,y=100,z=1115,distance=..40]", "gamerule spawn_mobs true",
           "gamerule advance_time true", "gamerule mob_griefing true")
        bot_stop()
    tot = lambda k: sum(len(s[k]) if isinstance(s[k], list) else s[k] for s in out)
    print(f"SUMMARY {label}: damage={tot('damage'):.1f} deaths={tot('deaths')} kills={tot('kills')}")


if __name__ == "__main__":
    run(sys.argv[1], sys.argv[2] if len(sys.argv) > 2 and sys.argv[2] != "-" else None,
        sys.argv[3] if len(sys.argv) > 3 else None)
