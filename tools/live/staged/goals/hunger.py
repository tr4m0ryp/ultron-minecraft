"""How often "only raw food" wakes thought, live. The body holds raw beef and nothing else
under a hunger effect, so the food bar keeps falling to the emergency level, one raw meal
lifts it again, and so on. Counted: `hungry_raw_food` events in the bot log by class (wake
or digest), and the beef the server saw eaten.

Usage: python3 hunger.py [--secs N] [--binary PATH]
One JSON line.
"""
import argparse, json, re, sys
from kit import *

BEEF = 48
# 0.1 exhaustion per tick: a food point every 2 s, a raw meal about every 9 s.
AMPLIFIER = 19
EVENT_RE = re.compile(r"^E (\d+) (Wake|Digest) hungry_raw_food")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--secs", type=int, default=240)
    ap.add_argument("--binary", default=None)
    a = ap.parse_args()
    build_site()
    start(binary=a.binary)
    try:
        prep(items=[("hotbar.1", "beef", BEEF)], comp_at=(CX - 12.5, Y + 1, CZ - 12.5))
        # Burn the full saturation and the first food points off quickly (to about 14).
        rc(f"effect give {NAME} hunger 4 255 true")
        time.sleep(4.5)
        n = len(log_lines())
        rc(f"effect give {NAME} hunger {a.secs + 30} {AMPLIFIER} true")
        time.sleep(a.secs)
        rc(f"effect clear {NAME}")
        left = count_item(NAME, "beef")
        lines = log_lines()[n:]
        events = [m.groups() for l in lines if (m := EVENT_RE.match(l))]
        ticks = [int(l.split()[1]) for l in lines if l.startswith("T ")]
        minutes = (ticks[-1] - ticks[0]) / 1200 if ticks else 0
        wakes = sum(1 for _, c in events if c == "Wake")
        print(json.dumps(dict(
            minutes=round(minutes, 2), beef_eaten=BEEF - left if isinstance(left, int) else None,
            wakes=wakes, digest_lines=len(events) - wakes,
            wakes_per_minute=round(wakes / minutes, 2) if minutes else None,
            wake_ticks=[int(t) for t, c in events if c == "Wake"])), flush=True)
    finally:
        stop()


if __name__ == "__main__":
    sys.exit(main())
