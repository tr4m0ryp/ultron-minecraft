"""Item 1: staged low-health zombie fights on the 21x21 edge platform.

Usage: python3 knockback.py <label> [fights] [binary]
Seeds alternate between a random placement and a back-to-the-edge placement (bot 1.5 blocks
from a random side, zombie 4 blocks inward). Prints one line per fight and a summary.
"""
import json, math, random, sys, time
from arena import *

X0, X1, Z0, Z1, Y = 1100, 1120, 990, 1010, 150


def placement(i):
    rng = random.Random(1000 + i)
    if i % 2 == 0:
        bx, bz = rng.uniform(X0 + 2, X1 - 1), rng.uniform(Z0 + 2, Z1 - 1)
        a = rng.uniform(0, 2 * math.pi)
        zx, zz = bx + 5 * math.cos(a), bz + 5 * math.sin(a)
        zx, zz = min(max(zx, X0 + 0.5), X1 + 0.5), min(max(zz, Z0 + 0.5), Z1 + 0.5)
    else:
        side = rng.randrange(4)
        t = rng.uniform(Z0 + 4, Z1 - 3)
        bx, bz, zx, zz = {
            0: (X0 + 1.5, t, X0 + 5.5, t), 1: (X1 - 0.5, t, X1 - 4.5, t),
            2: (t - Z0 + X0, Z0 + 1.5, t - Z0 + X0, Z0 + 5.5), 3: (t - Z0 + X0, Z1 - 0.5, t - Z0 + X0, Z1 - 4.5),
        }[side]
    return bx, bz, zx, zz


def fight(i, secs=30):
    bx, bz, zx, zz = placement(i)
    rc(f"kill @e[type=!player,x=1110,y=150,z=1000,distance=..40]", f"tp Ultron {bx:.2f} {Y} {bz:.2f}")
    prep_bot(night=True)
    # Drain food below 18 so natural regeneration stays off (the original low-health setup).
    rc("effect give Ultron hunger 4 255")
    time.sleep(4.5)
    rc("effect clear Ultron")
    time.sleep(0.5)
    rc("damage Ultron 13 minecraft:generic")
    time.sleep(0.3)
    L0, t0 = log_len(), now_ms()
    rc(f"summon zombie {zx:.2f} {Y} {zz:.2f} {{PersistenceRequired:1b,CanPickUpLoot:0b}}")
    time.sleep(secs)
    rs = rows(since_ms=t0)
    d = deaths(L0)
    rc(f"kill @e[type=zombie,x=1110,y=150,z=1000,distance=..120]")
    hp0 = rs[0]["hp"] if rs else None
    dmg = sum(max(0.0, a["hp"] - b["hp"]) for a, b in zip(rs, rs[1:]))
    fell = any(r["y"] < Y - 1.5 for r in rs)
    owners = {}
    for r in rs:
        for c in ("legs", "gaze", "main"):
            if r[c] != "-":
                owners[r[c]] = owners.get(r[c], 0) + 1
    res = dict(i=i, food=rs[0]["food"] if rs else None, hp0=hp0, min_hp=min((r["hp"] for r in rs), default=None), dmg=round(dmg, 1),
               fell=fell, died=bool(d), death=d[0] if d else "", min_y=min((r["y"] for r in rs), default=None),
               owners=owners)
    if d:
        time.sleep(3)
    return res


if __name__ in ("__main__",):
    label = sys.argv[1]
    n = int(sys.argv[2]) if len(sys.argv) > 2 else 10
    binary = sys.argv[3] if len(sys.argv) > 3 else None
    build_edge()
    bot_start(binary)
    out = []
    try:
        for i in range(n):
            r = fight(i)
            out.append(r)
            print(json.dumps(r), flush=True)
    finally:
        bot_stop()
    print(f"SUMMARY {label}: fights={len(out)} deaths={sum(r['died'] for r in out)} "
          f"falls={sum(r['fell'] for r in out)} dmg_total={sum(r['dmg'] for r in out):.1f} "
          f"causes={[r['death'] for r in out if r['died']]}")
