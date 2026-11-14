"""Item 1c: does water placed just before landing cancel fall damage on this server?

Usage: python3 mlg.py [reps]
Drops the bot (water bucket in hotbar.8, Resistance IV so failures are survivable and still
show as damage) from several heights onto stone, with R6's `mlg_lead` gain swept: R6 uses
the bucket on the first tick whose predicted ticks-to-landing is <= lead. Lead 0 never
places (control). Prints one JSON line per drop and a table of saves per lead.
"""
import json, sys, time
from arena import *

X, Z, FLOOR = 1153, 1153, 100
HEIGHTS = (20, 45)
LEADS = (0, 1, 2, 3, 4, 5, 6)


def build():
    rc("forceload add 1140 1140 1166 1166", f"fill 1143 {FLOOR - 2} 1143 1163 {FLOOR + 60} 1163 air",
       f"fill 1146 {FLOOR} 1146 1160 {FLOOR} 1160 stone")


def drop(height, lead):
    rc(f"fill 1146 {FLOOR + 1} 1146 1160 {FLOOR + 3} 1160 air", f"tp Ultron {X + 0.5} {FLOOR + 1} {Z + 0.5}")
    prep_bot(night=False, items=(("hotbar.8", "water_bucket", 1),))
    rc("item replace entity Ultron hotbar.0 with diamond_sword", "effect give Ultron resistance 30 3")
    send(f"gain mlg_lead {lead}")
    time.sleep(1.5)
    t0 = now_ms()
    rc(f"tp Ultron {X + 0.5} {FLOOR + 1 + height} {Z + 0.5} 0 0")
    time.sleep(4.5 + height / 40)
    rs = rows(since_ms=t0)
    if not rs:
        return None
    ev = [e for e in events(since_tick=rs[0]["tick"]) if "water_place" in e]
    land = first(rs, lambda r: r["y"] < FLOOR + 1.6)
    hp0 = max(r["hp"] for r in rs[:5])
    dmg = sum(max(0.0, a["hp"] - b["hp"]) for a, b in zip(rs, rs[1:]))
    wet = any(r["water"] == 1 for r in rs if land and r["tick"] >= land["tick"] - 1)
    back = rs[-1]["held"]
    return dict(height=height, lead=lead, hp0=hp0, dmg=round(dmg, 1), saved=dmg == 0, wet_at_landing=wet,
                place=ev[0].split("Log water_place R6 ")[-1] if ev else "", end_held=back)


if __name__ == "__main__":
    reps = int(sys.argv[1]) if len(sys.argv) > 1 else 2
    build()
    bot_start()
    out = []
    try:
        for lead in LEADS:
            for h in HEIGHTS:
                for _ in range(reps):
                    r = drop(h, lead)
                    if r:
                        out.append(r)
                        print(json.dumps(r), flush=True)
    finally:
        rc("effect clear Ultron")
        bot_stop()
    for lead in LEADS:
        for h in HEIGHTS:
            sel = [r for r in out if r["lead"] == lead and r["height"] == h]
            print(f"lead={lead} height={h}: saved {sum(r['saved'] for r in sel)}/{len(sel)} "
                  f"dmg={[r['dmg'] for r in sel]} place={[r['place'] for r in sel]}")
