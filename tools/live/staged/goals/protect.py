"""Placing into occupied cells, and protection of the player's own blocks, live.

Usage: python3 protect.py <binary|-> <trials> [case ...]
Cases (A, placing): place_leaves, place_dirt, place_grass, place_own. Each trial puts a
block of that kind in a cell, sets `place cobblestone <cell>` and checks the cell over RCON;
place_own has the bot place oak_planks first (its own block), then asks for cobblestone
without `replace` (must be refused, planks stay) and with it (must succeed).
Cases (B, protection) live in protect_hut.py: hut_go_to, hut_escape, hut_restart,
companion_wall. Run BEFORE with the baseline binary and AFTER with target/release.
One JSON line per trial; the site is cleared at the end.
"""
import json, os, sys
from kit import *

CELL = (CX + 4, Y + 1, CZ + 1)
HELD = dict(place_leaves="oak_leaves[persistent=true]", place_dirt="dirt", place_grass="grass_block")


def fresh_ledger():
    """The bot's own ledger file (staged runs never touch the play one)."""
    for who in ("bot", "comp"):
        try:
            os.remove(f"{S}/{who}.placed")
        except FileNotFoundError:
            pass


def run(spec, timeout=60):
    n = task(spec)
    kind, text, _ = outcome(n, timeout, "place")
    return kind, text


def clear_cell():
    rc(f"setblock {CELL[0]} {CELL[1]} {CELL[2]} air",
       f"kill @e[type=item,x={CX},y={Y},z={CZ},distance=..40]")


def place_soft(case):
    clear_cell()
    rc(f"setblock {CELL[0]} {CELL[1]} {CELL[2]} {HELD[case]}")
    prep(items=[("hotbar.1", "cobblestone", 8)], comp_at=(CX - 6.5, Y + 1, CZ - 6.5))
    kind, text = run("place cobblestone %d %d %d" % CELL)
    time.sleep(1)
    ok = kind == "goal_done" and is_block(*CELL, "cobblestone")
    return dict(ok=ok, outcome=kind, text=text)


def place_own():
    clear_cell()
    prep(items=[("hotbar.1", "oak_planks", 4), ("hotbar.2", "cobblestone", 8)],
         comp_at=(CX - 6.5, Y + 1, CZ - 6.5))
    k0, t0 = run("place oak_planks %d %d %d" % CELL)
    own = k0 == "goal_done" and is_block(*CELL, "oak_planks")
    time.sleep(2.5)
    k1, t1 = run("place cobblestone %d %d %d" % CELL, 30)
    refused = k1 == "goal_failed" and is_block(*CELL, "oak_planks")
    k2, t2 = run("place cobblestone %d %d %d replace" % CELL, 60)
    time.sleep(1)
    replaced = k2 == "goal_done" and is_block(*CELL, "cobblestone")
    return dict(ok=own and refused and replaced, placed_own=own, refused_without_replace=refused,
                replaced_with_replace=replaced, texts=[t0, t1, t2])


CASES = dict(place_leaves=lambda: place_soft("place_leaves"),
             place_dirt=lambda: place_soft("place_dirt"),
             place_grass=lambda: place_soft("place_grass"),
             place_own=place_own)


def main():
    binary = sys.argv[1] if len(sys.argv) > 1 and sys.argv[1] != "-" else None
    trials = int(sys.argv[2]) if len(sys.argv) > 2 else 5
    if binary:
        os.environ["MOTOR_BIN"] = binary
    from protect_hut import CASES as HUT
    cases = dict(CASES, **HUT)
    names = sys.argv[3:] or list(CASES)
    build_site()
    fresh_ledger()
    start(binary)
    try:
        for name in names:
            res = []
            for i in range(trials):
                r = cases[name]()
                r.update(case=name, trial=i + 1, binary=binary or "release")
                res.append(r)
                print(json.dumps(r), flush=True)
            print(json.dumps(dict(summary=name, ok=sum(1 for r in res if r.get("ok")), of=len(res),
                                  binary=binary or "release")), flush=True)
    finally:
        clear_cell()
        stop()
        from protect_hut import cleanup
        cleanup()


if __name__ == "__main__":
    main()
