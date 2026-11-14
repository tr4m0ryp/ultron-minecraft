"""Item 2: surfacing oscillation. A 17x17, 5-deep pool with a stone rim at y=120.

Usage: python3 swim.py <label> [binary]
  tread: dropped under water mid-pool (shore 8 blocks away, out of the 6-block sensing box)
         for 40 s; reports the y range after first surfacing and the minimum air.
  shore: dropped under water 3 blocks from the west rim; reports ticks until standing on
         the rim (y >= 121, on ground, out of water).
"""
import json, sys, time
from arena import *

def build():
    rc("forceload add 1296 996 1324 1024", "fill 1298 112 998 1322 126 1022 air",
       "fill 1299 114 999 1321 120 1021 stone", "fill 1302 115 1002 1318 120 1018 water")


def tread(secs=40):
    prep_bot(night=False)
    t0 = now_ms()
    rc("tp Ultron 1310.5 116 1010.5")
    time.sleep(secs)
    rs = rows(since_ms=t0)
    up = first(rs, lambda r: r["air"] == 300 and r["y"] > 119) or rs[0]
    after = [r for r in rs if r["tick"] >= up["tick"] + 20]
    ys = [r["y"] for r in after]
    under = sum(1 for r in after if r["air"] < 300) / max(1, len(after))
    owners = {}
    for r in after:
        owners[r["legs"]] = owners.get(r["legs"], 0) + 1
    return dict(case="tread", y_min=min(ys), y_max=max(ys), air_min=min(r["air"] for r in rs),
                frac_air_below_max=round(under, 2), owners=owners, hp_min=min(r["hp"] for r in rs))


def shore(secs=20):
    prep_bot(night=False)
    t0 = now_ms()
    rc("tp Ultron 1305.5 116 1010.5")
    time.sleep(secs)
    rs = rows(since_ms=t0)
    out = first(rs, lambda r: r["y"] >= 121 and r["ground"] == 1 and r["water"] == 0)
    return dict(case="shore", out_after_ticks=(out["tick"] - rs[0]["tick"]) if out else None,
                end=(round(rs[-1]["x"], 2), round(rs[-1]["y"], 2), round(rs[-1]["z"], 2)),
                air_min=min(r["air"] for r in rs))


if __name__ == "__main__":
    label = sys.argv[1]
    binary = sys.argv[2] if len(sys.argv) > 2 else None
    build()
    bot_start(binary)
    try:
        for f in (tread, shore):
            print(label, json.dumps(f()), flush=True)
    finally:
        bot_stop()
