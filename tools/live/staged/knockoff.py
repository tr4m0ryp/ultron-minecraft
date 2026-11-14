"""Item 1b/1c isolated: the bot stands 0.35 blocks from the east edge of the 21x21 platform
and does not fight or retreat (catalog limited to the listed ids); a zombie hits it from the
inner side until a hit knocks it off or 20 s pass. Measures whether R15 (air-strafe back)
and R6 (block clutch, cobblestone in the hotbar) save a knock-off.

Usage: python3 knockoff.py <label> <only-ids> [trials] [cobble]
"""
import json, sys, time
from arena import *

Y, EDGE_X = 150, 1121.0


def trial(i, cobble):
    z = 994.5 + (i % 4) * 4
    rc("kill @e[type=zombie,x=1110,y=150,z=1000,distance=..120]",
       "fill 1121 140 990 1125 149 1010 air", f"tp Ultron {EDGE_X - 0.35:.2f} {Y} {z} 90 0")
    items = [("hotbar.0", "diamond_sword", 1)] + ([("hotbar.2", "cobblestone", 32)] if cobble else [])
    prep_bot(night=True, items=items)
    rc("effect give Ultron resistance 60 2")
    time.sleep(1.5)
    L0, t0 = log_len(), now_ms()
    rc(f"summon zombie {EDGE_X - 3.5:.2f} {Y} {z} {{PersistenceRequired:1b,CanPickUpLoot:0b}}")
    end = time.time() + 20
    while time.time() < end:
        time.sleep(0.5)
        rs = rows(since_ms=t0)
        if rs and rs[-1]["y"] < Y - 6:
            break
    time.sleep(4.0 if rs and rs[-1]["y"] < Y - 6 else 0)
    rs = rows(since_ms=t0)
    hits = sum(1 for e in events(since_tick=rs[0]["tick"]) if e.startswith("D ")) if rs else 0
    off = [r for r in rs if r["x"] > EDGE_X + 0.3]
    fell = any(r["y"] < Y - 1.5 for r in rs)
    placed = rc("execute if block 1121 149 %d cobblestone" % int(z))
    owners = {}
    for r in off:
        owners[r["legs"]] = owners.get(r["legs"], 0) + 1
        owners["main:" + r["main"]] = owners.get("main:" + r["main"], 0) + 1
    return dict(i=i, hits=hits, went_over_edge=bool(off), max_x=round(max((r["x"] for r in rs), default=0), 2),
                fell=fell, died=bool(deaths(L0)), clutch_block="passed" in placed, owners_off_edge=owners)


if __name__ == "__main__":
    label, only = sys.argv[1], sys.argv[2]
    n = int(sys.argv[3]) if len(sys.argv) > 3 else 8
    cobble = len(sys.argv) > 4 and sys.argv[4] == "cobble"
    build_edge()
    bot_start(only=only)
    out = []
    try:
        for i in range(n):
            r = trial(i, cobble)
            out.append(r)
            print(label, json.dumps(r), flush=True)
    finally:
        rc("kill @e[type=zombie,x=1110,y=150,z=1000,distance=..120]", "effect clear Ultron")
        bot_stop()
    over = [r for r in out if r["went_over_edge"]]
    print(f"SUMMARY {label}: trials={len(out)} over_edge={len(over)} fell={sum(r['fell'] for r in out)} "
          f"saved={sum(1 for r in over if not r['fell'])} deaths={sum(r['died'] for r in out)} "
          f"clutch_blocks={sum(r['clutch_block'] for r in out)}")
