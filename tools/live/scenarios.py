"""Usage: python3 tools/live/scenarios.py <lava|lava_adj|fire|cliff|air|eat|creeper|zombie|zombie_low|stuck> [seconds]."""
import sys, time
from harness import *

def summarize(t0, rs, owner_ids, extra=""):
    base = rs[0]["tick"] if rs else None
    hit = first(rs, lambda r: any(r[c] in owner_ids for c in ("legs", "gaze", "main", "inv")))
    lat = (hit["tick"] - base) if hit else None
    minhp = min(r["hp"] for r in rs) if rs else None
    print(f"base_tick={base} fired={'yes' if hit else 'no'} first_owner_tick={hit and hit['tick']} "
          f"latency_ticks={lat} min_hp={minhp} end={rs[-1] if rs else None} {extra}")

def watch(seconds):
    time.sleep(seconds)

def show(rs, every=5, keys=("tick","x","y","z","hp","food","air","fire","legs","gaze","main","inv","held")):
    for r in rs[::every]:
        print(" ".join(f"{k}={r[k]}" for k in keys))

name = sys.argv[1]
L0 = log_len()
if name == "lava":
    reset(); t0 = now_ms(); rc("setblock 1000 150 1000 lava"); watch(20)
    rs = rows(since_ms=t0); show(rs[:30], 2); summarize(t0, rs, {"R1"})
    rc("fill 990 150 990 1010 152 1010 air")
elif name == "lava_adj":
    reset(); t0 = now_ms(); rc("setblock 1001 150 1000 lava"); watch(5)
    rs = rows(since_ms=t0); show(rs[:80], 4); summarize(t0, rs, {"R1"})
    rc("fill 990 150 990 1010 152 1010 air")
elif name == "fire":
    reset(); t0 = now_ms(); rc("setblock 1000 150 1000 fire"); watch(4)
    rs = rows(since_ms=t0); show(rs[:30], 2); summarize(t0, rs, {"R4", "R3"})
    rc("fill 990 150 990 1010 152 1010 air")
elif name == "cliff":
    reset(1035.5, 150, 1000.5); t0 = now_ms(); send("goal 1050.5 150 1000.5 0.6 walk east"); watch(6)
    rs = rows(since_ms=t0); show(rs[:80], 4); summarize(t0, rs, {"R5"}, f"max_x={max(r['x'] for r in rs):.2f} min_y={min(r['y'] for r in rs):.2f}")
    send("goal-end")
elif name == "air":
    reset(); rc("fill 995 142 995 997 149 997 water"); time.sleep(0.5)
    t0 = now_ms(); rc("tp Ultron 996.5 142 996.5"); watch(int(sys.argv[2]) if len(sys.argv) > 2 else 20)
    rs = rows(since_ms=t0); show(rs, 10)
    trig = first(rs, lambda r: r["air"] < 100)
    hit = first(rs, lambda r: r["legs"] == "R8")
    print("air<100 tick", trig and trig["tick"], "R8 tick", hit and hit["tick"], "min_air", min(r["air"] for r in rs), "min_hp", min(r["hp"] for r in rs))
    rc("fill 995 142 995 997 149 997 stone")
elif name == "eat":
    reset(); rc("item replace entity Ultron hotbar.3 with bread 8", "effect give Ultron hunger 30 120"); t0 = now_ms(); watch(int(sys.argv[2]) if len(sys.argv) > 2 else 25)
    rs = rows(since_ms=t0); show([r for r in rs if r["food"] <= 15], 4)
    trig = first(rs, lambda r: r["food"] <= 14)
    hit = first(rs, lambda r: r["main"] == "H1" or r["inv"] == "H1")
    print("food<=14 tick", trig and trig["tick"], "H1 tick", hit and hit["tick"], "min_food", min(r["food"] for r in rs), "end_food", rs[-1]["food"], "held", rs[-1]["held"])
elif name == "creeper":
    reset(); t0 = now_ms(); rc("summon creeper 1000.5 150 1006.5"); watch(int(sys.argv[2]) if len(sys.argv) > 2 else 12)
    rs = rows(since_ms=t0); show(rs, 5); summarize(t0, rs, {"D1"})
    print(events(since_tick=rs[0]["tick"])[:15])
elif name in ("zombie", "zombie_low"):
    reset(night=True); rc("item replace entity Ultron hotbar.0 with diamond_sword")
    if name == "zombie_low":
        rc("damage Ultron 13 minecraft:generic")
        time.sleep(0.3)
    t0 = now_ms(); rc("summon zombie 1000.5 150 1006.5"); watch(int(sys.argv[2]) if len(sys.argv) > 2 else 15)
    rs = rows(since_ms=t0); show(rs, 5); summarize(t0, rs, {"D2", "H2", "K1"})
    print(rc("execute if entity @e[type=zombie,x=1000,y=150,z=1000,distance=..40]"))
    print(events(since_tick=rs[0]["tick"])[:20])
elif name == "stuck":
    reset(); rc("fill 1003 150 990 1003 151 1010 dirt"); t0 = now_ms(); send("goal 1008.5 150 1000.5 0.6 walk east"); watch(int(sys.argv[2]) if len(sys.argv) > 2 else 20)
    rs = rows(since_ms=t0); show(rs, 5); summarize(t0, rs, {"C6"}, f"max_x={max(r['x'] for r in rs):.2f}")
    print(events(since_tick=rs[0]["tick"])[:20])
    send("goal-end")
print("deaths:", server_deaths(L0))
