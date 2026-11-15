"""Print the result tables from results/rerun (baseline repetitions) and results/sweep (variants).

  python summarize.py
"""
import glob, json, os, statistics
HERE = os.path.dirname(os.path.abspath(__file__))
CLEAN = 1.0  # cores used by other processes during a run; above this (or with a compiler seen) the run is marked "!"


def load(sub):
    return [json.load(open(f)) for f in sorted(glob.glob(os.path.join(HERE, "results", sub, "*.json")))]


def env(r):
    i, d = r["idle"], r["during"]
    flag = "!" if d["compilers_seen"] or i["compilers"] or d["foreign_cores_mean"] > CLEAN else ""
    return f"{i['load1']} | {d['foreign_cores_mean']}{flag}"


def main():
    print("baseline rerun\n| run | threads | correct | p50 ms | p90 ms | n | batch5 p50 ms | load1 | foreign cores |\n|---|---|---|---|---|---|---|---|---|")
    groups = {}
    for r in load("rerun"):
        s = r["single_ms"]
        b = r["batch5_ms"]["p50"] if r["batch5_ms"] else "-"
        print(f"| {r['label']} | {r['threads']} | {r['correct']}/{r['n_cases']} | {s['p50']} | {s['p90']} | {s['n']} | {b} | {env(r)} |")
        groups.setdefault(r["label"].rsplit("-r", 1)[0], []).append(r)
    print("\nmean and spread over repetitions\n| model | reps | p50 ms mean (min to max) | correct per rep |\n|---|---|---|---|")
    for g, rs in groups.items():
        p = [r["single_ms"]["p50"] for r in rs]
        print(f"| {g} | {len(rs)} | {statistics.mean(p):.0f} ({min(p):.0f} to {max(p):.0f}) | {', '.join(str(r['correct']) + '/' + str(r['n_cases']) for r in rs)} |")
    print("\nvariants\n| run | cases | threads | prompt tok | suffix tok | argmax | calib | generate | cold p50 | cold p90 | cached p50 | cached p90 | gen p50 | n | load1 | foreign cores |\n" + "|---" * 16 + "|")
    for r in load("sweep"):
        if "cached_ms" not in r:
            continue
        n, c, k, g = r["n_cases"], r["cold_ms"], r["cached_ms"], r["generate_ms"]
        f = lambda v: "-" if v is None else f"{v}/{n}"
        print(f"| {r['label']} | {r['cases']} | {r['threads']} | {r['prompt_tokens_mean']} | {r['suffix_tokens_mean']} | {f(r['correct_argmax'])} | "
              f"{f(r['correct_calib'])} | {f(r['correct_generate'])} | {c['p50'] if c else '-'} | {c['p90'] if c else '-'} | {k['p50']} | {k['p90']} | {g['p50'] if g else '-'} | {k['n']} | {env(r)} |")
    print("\nsentence encoder\n| run | cases | threads | zero-shot | nearest example | leave-one-out | p50 ms | p90 ms | n | load1 | foreign cores |\n" + "|---" * 11 + "|")
    for r in load("sweep"):
        if "ms" in r:
            n = r["n_cases"]
            f = lambda v: "-" if v is None else f"{v}/{n}"
            print(f"| {r['label']} | {r['cases']} | {r['threads']} | {f(r['correct_zero'])} | {f(r['correct_shots'])} | {f(r.get('correct_loo'))} | "
                  f"{r['ms']['p50']} | {r['ms']['p90']} | {r['ms']['n']} | {env(r)} |")


if __name__ == "__main__":
    main()
