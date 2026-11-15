"""Kill test for the ledger of placed blocks. The bot places one dirt block by a `place`
goal and prints `LEDGER own x y z dirt t=<tick>` in the tick the server confirms it. The
script kills the bot the instant it reads that line and then looks at the ledger file: the
cell must be in it. The block itself is checked on the server.

Usage: python3 ledger_kill.py <binary|-> [trials]
One JSON line per trial and a summary line; the site is cleared at the end.
"""
import json, os, signal, sys
from pathlib import Path
from kit import *
from terrain import clear

CELL = (CX + 2, Y + 1, CZ)
LINE = "LEDGER own %d %d %d dirt" % CELL
OWN = "own %d %d %d dirt" % CELL


def kill_bot():
    try:
        os.killpg(int(Path(f"{S}/bot.pid").read_text()), signal.SIGKILL)
    except ProcessLookupError:
        pass


def trial(binary):
    Path(f"{S}/bot.placed").unlink(missing_ok=True)
    start(binary, companion=False)
    clear()
    prep(items=[("hotbar.0", "dirt", 4)])
    size = os.path.getsize(f"{S}/bot.log")
    send("task place dirt %d %d %d" % CELL)
    seen, end = False, time.time() + 30
    with open(f"{S}/bot.log", errors="replace") as f:
        f.seek(size)
        buf = ""
        while time.time() < end and not seen:
            buf += f.read()
            seen = LINE in buf
            if not seen:
                time.sleep(0.001)
    kill_bot()
    time.sleep(0.5)
    ledger = Path(f"{S}/bot.placed")
    on_disk = ledger.exists() and OWN in ledger.read_text().splitlines()
    placed = is_block(*CELL, "dirt")
    stop()
    return dict(ok=seen and placed and on_disk, confirmed_line=seen, on_server=placed,
                in_file=on_disk)


if __name__ == "__main__":
    binary = None if sys.argv[1] == "-" else sys.argv[1]
    trials = int(sys.argv[2]) if len(sys.argv) > 2 else 10
    os.environ["ULTRON_BRIDGE"] = "0"
    build_site()
    wins = files = 0
    try:
        for k in range(trials):
            r = trial(binary)
            wins += r["ok"]
            files += r["in_file"]
            print(json.dumps(dict(trial=k + 1, **r)), flush=True)
        print(json.dumps(dict(summary="ledger_kill", ok=wins, in_file=files, of=trials)), flush=True)
    finally:
        stop()
        clear()
