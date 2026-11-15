"""The `protect` tool live, over the real tool path (brain-link-mcp on stdio, the bot's socket
behind it). A sealed 5x5 dirt hut is built by RCON around the bot, so none of its cells are
in the bot's ledger of blocks it placed; only the tool can keep them.

Cases: protect_tool (mark the hut box with `protect`, then `go_to` a point outside: no hut
cell may break; the goal may fail or the body may tunnel under the floor), unprotected
(control: the same trial without `protect`, where the route digs through the wall).
Each trial lifts the box again with `protect clear`.

Usage: python3 protect_tool.py [-n trials] [case ...]   (needs the bot's bridge socket free)
One JSON line per trial and a summary line per case; the site is cleared at the end.
"""
import argparse, json, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from mcp_drive import Mcp  # noqa: E402
from protect_hut import (EXIT, HX0, HX1, HZ0, HZ1, ROOF, hut_by_rcon, missing,  # noqa: E402
                         newly_broken, prep_inside, reset_site)
from kit import *  # noqa: E402,F401


def connect():
    m = Mcp()
    m.rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                         "clientInfo": {"name": "protect_tool", "version": "0"}})
    m.p.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n")
    m.p.stdin.flush()
    return m


def trial(m, protect):
    reset_site()
    hut_by_rcon()
    before = missing()
    prep_inside()
    ack = None
    if protect:
        box = {"min": [HX0, Y + 1, HZ0], "max": [HX1, ROOF, HZ1]}
        ack, err, _ = m.tool("protect", box)
        if err:
            return dict(ok=False, protect_ack=ack)
    t0 = time.time()
    n = task("go_to %s %s %s" % EXIT)
    kind, text, _ = outcome(n, 120, "go_to")
    p = pos()
    broken = newly_broken(before)
    cleared, _, _ = m.tool("protect", {"clear": [HX0, Y + 1, HZ0]})
    ok = not before and (broken == 0 if protect else broken > 0)
    return dict(ok=ok, hut_broken=broken, protect_ack=ack, cleared=cleared, outcome=kind,
                text=text, secs=round(time.time() - t0, 1), pos=p)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-n", type=int, default=5)
    ap.add_argument("cases", nargs="*")
    a = ap.parse_args()
    names = a.cases or ["protect_tool", "unprotected"]
    build_site()
    start()
    m = connect()
    try:
        for name in names:
            wins = 0
            for i in range(a.n):
                r = trial(m, name == "protect_tool")
                wins += r["ok"]
                print(json.dumps(dict(r, case=name, trial=i + 1)), flush=True)
            print(json.dumps(dict(summary=name, ok=wins, of=a.n)), flush=True)
    finally:
        m.p.terminate()
        reset_site()
        rc(f"fill {HX0} {Y + 1} {HZ0} {HX1} {ROOF} {HZ1} air")
        stop()


if __name__ == "__main__":
    sys.exit(main())
