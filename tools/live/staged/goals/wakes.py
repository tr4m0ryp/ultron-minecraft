"""Wakes per goal when the body reports a block, live. Three cases on the goal site:
sealed (a go_to into a bedrock shell: path problems, then the goal fails), mine_mixed (two
iron ore blocks deep in bedrock nearest the body and two in the open: the sealed ones are
skipped, the open ones mined) and follow_gone (the companion out of sight: a block only
thought can clear, which must still wake).

Counts, per trial, the WAKE lines the bot printed between the goal and its outcome, the
goal_blocked ones among them, and the goal_blocked events that went to the digest instead
(`E <tick> Digest goal_blocked` trace lines). The outcome is checked on the server: the
body's position for sealed, the mined ore for mine_mixed.

Usage: python3 wakes.py <binary|-> <trials> [case ...]
"""
import json, signal, sys
from kit import *
from terrain import clear


def cell(dx, dy, dz):
    return X0 + dx, Y + dy, Z0 + dz


def shell(dx, dz, inner, r=1):
    """A bedrock box of half-width `r` round one cell on the floor. The hands dig a block in
    reach through a wall, so a block that must stay out of reach needs r = 4."""
    x, y, z = cell(dx, 1, dz)
    return [f"fill {x - r} {y} {z - r} {x + r} {y + r + 1} {z + r} bedrock",
            f"setblock {x} {y} {z} {inner}"]


def counts(since):
    lines = log_lines()[since:]
    wakes = [l for l in lines if l.startswith("WAKE ")]
    return dict(wakes=len(wakes), blocked_wakes=sum(" goal_blocked " in l for l in wakes),
                blocked_digest=sum(" Digest goal_blocked " in l for l in lines if l.startswith("E ")))


def sealed():
    rc(*shell(20, 20, "air"))
    prep(at=(X0 + 4.5, Y + 1, Z0 + 20.5))
    x, y, z = cell(20, 1, 20)
    n = task(f"go_to {x + 0.5} {y} {z + 0.5}")
    kind, text, _ = outcome(n, 90, "go_to")
    time.sleep(1)
    p = pos()
    return dict(ok=kind == "goal_failed" and dist(p, (x + 0.5, y, z + 0.5)) > 1.5, outcome=kind,
                text=text, **counts(n))


def mine_mixed():
    ores = [cell(30, 1, 16), cell(30, 1, 24)]
    rc(*shell(8, 10, "iron_ore", 4), *shell(8, 30, "iron_ore", 4),
       *[f"setblock {x} {y} {z} iron_ore" for x, y, z in ores])
    prep(items=[("hotbar.0", "stone_pickaxe", 1)], at=(X0 + 8.5, Y + 1, Z0 + 20.5))
    n = task("mine iron_ore 2")
    kind, text, _ = outcome(n, 150, "mine")
    time.sleep(1)
    gone = sum(not is_block(x, y, z, "iron_ore") for x, y, z in ores)
    return dict(ok=kind == "goal_done" and gone == 2, outcome=kind, text=text,
                open_ore_mined=gone, **counts(n))


def follow_gone():
    prep(at=(X0 + 4.5, Y + 1, Z0 + 20.5), comp_at=(X0 + 4.5, Y + 250, Z0 + 20.5 + 400))
    time.sleep(3)
    n = task("follow companion 3")
    time.sleep(12)
    c = counts(n)
    send("goal-end")
    return dict(ok=c["blocked_wakes"] >= 1, outcome="waited 12 s", text=None, **c)


CASES = dict(sealed=sealed, mine_mixed=mine_mixed, follow_gone=follow_gone)

if __name__ == "__main__":
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(1))
    binary, trials = sys.argv[1], int(sys.argv[2])
    names = sys.argv[3:] or list(CASES)
    build_site()
    start(None if binary == "-" else binary)
    try:
        for name in names:
            tot = dict(wakes=0, blocked_wakes=0, blocked_digest=0, ok=0)
            for k in range(trials):
                clear()
                r = CASES[name]()
                for key in tot:
                    tot[key] += int(r[key])
                print(json.dumps(dict(case=name, trial=k, **r)), flush=True)
                send("goal-end")
                time.sleep(1)
            print(json.dumps(dict(summary=name, trials=trials, **tot)), flush=True)
    finally:
        stop()
        clear()
        build_site()
