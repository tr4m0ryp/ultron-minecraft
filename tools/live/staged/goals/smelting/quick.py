"""Blast furnace and smoker, live: the furnace's menu at twice the speed.

Usage: python3 smelting/quick.py <binary|-> <trials> [case ...]

A blast furnace or a smoker stands 4 blocks from the body and no furnace anywhere; the kit
holds nothing a furnace could be made from. Counts are read on the server; the time is
from the goal to its outcome (a furnace takes 10 s an item, these 5 s).
  blast     2 raw iron and 1 coal, `smelt iron_ingot 2`
  smoke     2 beef and 1 coal, `smelt cooked_beef 2`
  furnace   the same iron at a furnace, for the time
  not_blast 2 oak logs at the blast furnace, `smelt charcoal 1`: charcoal is not blasted,
            so one failure asking for a furnace, the logs kept
"""
import json, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from kit import *

AT = (CX + 4, Y + 1, CZ)
KINDS = ("blast_furnace", "smoker", "furnace")


def have(item):
    n = count_item(NAME, item)
    return n if isinstance(n, int) else -1


def site(block):
    rc(f"fill {X0} {Y - 3} {Z0} {X1} {Y} {Z1} stone",
       *[f"fill {X0} {Y + 1} {Z0} {X1} {Y + 3} {Z1} air replace minecraft:{k}" for k in KINDS],
       f"kill @e[type=item,x={CX},y={Y},z={CZ},distance=..40]",
       "setblock %d %d %d minecraft:%s" % (*AT, block))


def smelt(block, kit, spec, item, n, timeout=120):
    site(block)
    prep(items=kit)
    t0 = time.time()
    k = task(spec)
    kind, text, wakes = outcome(k, timeout, "smelt")
    secs = round(time.time() - t0, 1)
    time.sleep(1.5)
    got = have(item)
    others = sum("Successfully" in rc(f"fill {X0} {Y + 1} {Z0} {X1} {Y + 3} {Z1} air replace minecraft:{b}")
                 for b in KINDS if b != block)
    return dict(outcome=kind, text=text, got=got, secs=secs, other_stations_put_down=others,
                failures=sum(1 for w in wakes if w[1] == "goal_failed"), ok=kind == "goal_done" and got == n and others == 0)


IRON = [("hotbar.1", "raw_iron", 2), ("hotbar.2", "coal", 1)]


def not_blast():
    r = smelt("blast_furnace", [("hotbar.1", "oak_log", 2)], "smelt charcoal 1", "charcoal", 0, timeout=60)
    logs = have("oak_log")
    asks = "need 1 furnace, have 0" in (r["text"] or "")
    r.update(ok=r["outcome"] == "goal_failed" and r["failures"] == 1 and asks and logs == 2 and r["got"] == 0,
             asks_for_furnace=asks, logs_left=logs)
    return r


CASES = dict(
    blast=lambda: smelt("blast_furnace", IRON, "smelt iron_ingot 2", "iron_ingot", 2),
    smoke=lambda: smelt("smoker", [("hotbar.1", "beef", 2), ("hotbar.2", "coal", 1)], "smelt cooked_beef 2",
                        "cooked_beef", 2),
    furnace=lambda: smelt("furnace", IRON, "smelt iron_ingot 2", "iron_ingot", 2),
    not_blast=not_blast)

if __name__ == "__main__":
    binary = None if sys.argv[1] == "-" else sys.argv[1]
    names = sys.argv[3:] or list(CASES)
    build_site()
    start(binary, companion=False)
    try:
        for name in names:
            rs = []
            for i in range(int(sys.argv[2])):
                r = CASES[name]()
                rs.append(r)
                print(json.dumps(dict(case=name, trial=i + 1, **r)), flush=True)
            print(f"SUMMARY {name}: {sum(r['ok'] for r in rs)}/{len(rs)}, secs {[r['secs'] for r in rs]}", flush=True)
    finally:
        rc(*[f"fill {X0} {Y + 1} {Z0} {X1} {Y + 3} {Z1} air replace minecraft:{k}" for k in KINDS])
        stop()
