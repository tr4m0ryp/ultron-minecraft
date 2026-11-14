"""Summarise survival.py outputs: mean and sample sd of deaths, HP lost and kills per wave.

Usage: python3 stats.py <arm>=<glob> [<arm>=<glob> ...]
  e.g. python3 stats.py full='logs/noise/full_*.txt' melee='logs/noise/melee_*.txt'
"""
import glob, json, statistics as st, sys

WAVES = ["zombies", "skeletons", "creepers", "mix"]


def load(pattern):
    runs = []
    for path in sorted(glob.glob(pattern)):
        waves = {}
        for line in open(path, errors="replace"):
            head, _, rest = line.partition(" ")
            if rest.startswith("{"):
                w = json.loads(rest)
                waves[w["wave"]] = w
        if waves:
            runs.append(waves)
    return runs


def ms(xs):
    if not xs:
        return "-"
    sd = st.stdev(xs) if len(xs) > 1 else 0.0
    return f"{st.mean(xs):.1f} +- {sd:.1f}"


def summarise(arm, runs):
    print(f"{arm}: {len(runs)} runs")
    print(f"  {'wave':10} {'n':>2} {'deaths':>12} {'hp_lost':>14} {'kills':>12}")
    for wave in WAVES + ["total"]:
        rows = [r for r in runs if wave in r] if wave != "total" else runs
        if not rows:
            continue
        pick = (lambda r, k: r[wave][k]) if wave != "total" else (
            lambda r, k: sum(w[k] if not isinstance(w[k], list) else len(w[k]) for w in r.values()))
        d = [len(r[wave]["deaths"]) if wave != "total" else pick(r, "deaths") for r in rows]
        h = [pick(r, "damage") for r in rows]
        k = [pick(r, "kills") for r in rows]
        print(f"  {wave:10} {len(rows):>2} {ms(d):>12} {ms(h):>14} {ms(k):>12}")


if __name__ == "__main__":
    for spec in sys.argv[1:]:
        arm, _, pattern = spec.partition("=")
        summarise(arm, load(pattern))
