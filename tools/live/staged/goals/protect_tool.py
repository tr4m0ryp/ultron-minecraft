"""The `protect` tool live, over the real tool path (brain-link-mcp on stdio, the bot's socket
behind it). A sealed 5x5 dirt hut is built by RCON around the bot, so none of its cells are
in the bot's ledger of blocks it placed; only the tool can keep them.

Cases: protect_tool (mark the hut box with `protect`, then `go_to` a point outside: no hut
cell may break; the goal may fail or the body may tunnel under the floor), protect_restart
(the same, with the bot killed the instant `protect` is acknowledged and started again: the
box must be in the ledger file once the bot is dead and come back from it), unprotected
(control: no `protect`; the route digs through a wall). Each trial lifts the box again with
`protect clear`.

Usage: python3 protect_tool.py [-n trials] [case ...]   (needs the bot's bridge socket free)
One JSON line per trial and a summary line per case; the site is cleared at the end.
"""
import argparse, json, os, signal, sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from mcp_drive import Mcp  # noqa: E402
from protect_hut import (EXIT, HX0, HX1, HZ0, HZ1, ROOF, hut_by_rcon, missing,  # noqa: E402
                         newly_broken, prep_inside, reset_site)
from kit import *  # noqa: E402,F401


REGION = f"region {HX0} {Y + 1} {HZ0} {HX1} {ROOF} {HZ1}"


def kill_bot():
    """SIGTERM to the bot's process group, as `bot.sh stop` sends it, without the shell."""
    try:
        os.killpg(int(Path(f"{S}/bot.pid").read_text()), signal.SIGTERM)
    except ProcessLookupError:
        pass


def connect():
    m = Mcp()
    m.rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                         "clientInfo": {"name": "protect_tool", "version": "0"}})
    m.p.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n")
    m.p.stdin.flush()
    return m


def trial(m, protect, restart=False):
    reset_site()
    hut_by_rcon()
    before = missing()
    prep_inside()
    ack = None
    if protect:
        box = {"min": [HX0, Y + 1, HZ0], "max": [HX1, ROOF, HZ1]}
        ack, err, _ = m.tool("protect", box)
        if err:
            return m, dict(ok=False, protect_ack=ack)
    on_disk = None
    if restart:
        # Killed the instant the acknowledgement is read: the box must already be in the
        # ledger file (the reply used to leave a few ms before the write: 1 box in 5 lost).
        kill_bot()
        time.sleep(0.5)
        on_disk = REGION in Path(f"{S}/bot.placed").read_text().splitlines()
        m.p.terminate()
        stop()
        start()
        m = connect()
        prep_inside()
    t0 = time.time()
    n = task("go_to %s %s %s" % EXIT)
    kind, text, _ = outcome(n, 120, "go_to")
    p = pos()
    broken = newly_broken(before)
    cleared, _, _ = m.tool("protect", {"clear": [HX0, Y + 1, HZ0]})
    ok = not before and (broken == 0 if protect else broken > 0)
    return m, dict(ok=ok, hut_broken=broken, protect_ack=ack, box_on_disk=on_disk, cleared=cleared,
                   outcome=kind, text=text, secs=round(time.time() - t0, 1), pos=p)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("-n", type=int, default=5)
    ap.add_argument("cases", nargs="*")
    a = ap.parse_args()
    names = a.cases or ["protect_tool", "protect_restart", "unprotected"]
    build_site()
    start()
    m = connect()
    try:
        for name in names:
            wins = 0
            for i in range(a.n):
                m, r = trial(m, name != "unprotected", name == "protect_restart")
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
