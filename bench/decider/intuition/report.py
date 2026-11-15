"""Print the result tables of the trained-Intuition measurement from results/trained/*.json.

  python report.py
"""
import glob, json, os
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(os.path.dirname(HERE), "results", "trained")


def load(name):
    p = os.path.join(OUT, name + ".json")
    return json.load(open(p)) if os.path.exists(p) else None


def cell(r):
    return f"{r['k']}/{r['n']} ({r['ci95'][0]} to {r['ci95'][1]})" if r else "n/a"


def cov(r, key="cov95_oracle"):
    c = r.get(key) if r else None
    return "n/a" if not c else (f"{c['taken']} ({c['coverage']} %)" if c["taken"] else "0")


def mcnemar(a, b, gold):
    """Exact two-sided sign test on the cases where exactly one of the two arms is right. -> (a only, b only, p)."""
    from math import comb
    x = sum(p == g != q for p, q, g in zip(a, b, gold)); y = sum(q == g != p for p, q, g in zip(a, b, gold))
    n, k = x + y, min(x, y)
    p = min(1.0, 2 * sum(comb(n, i) for i in range(k + 1)) / 2 ** n) if n else 1.0
    return x, y, round(p, 4)


def main():
    fl, cl, pl, hb = load("floors"), load("classic"), load("classic-plan"), load("habit")
    print("| arm | trained on | test_real /108 (95 % interval) | test_hand /48 | leave-one-session-out /470 | taken at 95 % "
          "precision, test_real: oracle threshold | same, threshold from training day |\n|---|---|---|---|---|---|---|")
    if fl:
        r, h, al = fl["rows"]["test_real"], fl["rows"]["test_hand"], fl["rows"]["all_real"]
        print(f"| uniform chance over {fl['chance_options']} options | - | {r['chance']} | {h['chance']} | {al['chance']} | - | - |")
        for key, name in (("majority", f"majority label of the training day ({fl['majority_label']})"),
                          ("repeat_last", "repeat the last decision"), ("keyword_rule", "keyword rule of 9.2")):
            print(f"| {name} | - | {cell(r[key])} | {cell(h[key])} | {cell(al[key])} | - | - |")
        e = "encoder_nearest_of_12"
        print(f"| encoder, nearest of 12 solved examples (9.4) | 12 | {cell(r[e])} | {cell(h[e])} | n/a | {cov(r[e])} | n/a |")
    for f in sorted(glob.glob(os.path.join(OUT, "untrained-*-real.json"))):
        a, b = json.load(open(f)), load(os.path.basename(f)[:-10] + "-hand")
        ms = a["cached_ms"]
        print(f"| {a['label'][:-5]}, 12 examples cached, argmax (calibrated) | 12 | {a['correct_argmax']}/108 "
              f"({a['correct_calib']}/108) | {b['correct_argmax'] if b else '?'}/48 ({b['correct_calib'] if b else '?'}/48) "
              f"| n/a | n/a | n/a | p50 {ms['p50']} p90 {ms['p90']} ms")
    for src, tag in ((cl, ""), (pl, " + plan line"), (hb, "")):
        for k, v in (src or {"arms": {}})["arms"].items():
            if k.endswith("| detail"):
                continue
            parts = [x.strip() for x in k.split("|")]
            print(f"| {parts[0]}{tag} | {parts[1]} | {cell(v['test_real'])} | {cell(v['test_hand'])} | "
                  f"{cell(v.get('loso'))} | {cov(v['test_real'])} | {cov(v['test_real'], 'cov95_train_threshold')} |")
    for f in sorted(glob.glob(os.path.join(OUT, "encoder-ft-*.json")) + glob.glob(os.path.join(OUT, "lm135-lora-*.json"))):
        v = json.load(open(f))
        print(f"| {v['arm']} | {v['set']} ({v['epochs']} epochs, {v['train_s']} s) | {cell(v['test_real'])} | "
              f"{cell(v['test_hand'])} | n/a | {cov(v['test_real'])} | n/a |")
    print("\naccuracy when acting only above a confidence, test_real (taken: precision %)\n| arm | " +
          " | ".join(f"t={t}" for t in (0.5, 0.6, 0.7, 0.8, 0.9, 0.95)) + " |\n|---|---|---|---|---|---|---|")
    rows = [(k, v["test_real"]) for k, v in (cl or {"arms": {}})["arms"].items() if k.endswith("| real")]
    rows += [(k, v["test_real"]) for k, v in (hb or {"arms": {}})["arms"].items() if k.endswith("real | label")]
    rows += [(json.load(open(f))["arm"] + " | " + json.load(open(f))["set"], json.load(open(f))["test_real"])
             for f in sorted(glob.glob(os.path.join(OUT, "*-ft-*.json")) + glob.glob(os.path.join(OUT, "lm135-lora-*.json")))]
    for k, t in rows:
        print(f"| {k} | " + " | ".join(f"{c['taken']}: {c['precision'] if c['taken'] else '-'}" for c in t["curve"][1:]) + " |")
    if cl:
        import sys
        sys.path.insert(0, HERE)
        import data
        gold = [p["label"] for p in data.real()[1]]
        a = cl["arms"]["a1 encoder + logistic head | real"]
        print("\npaired comparison on test_real and leave-one-session-out (first only right, second only right, p)")
        for other in ("c1 features + logistic regression | real", "c2 features + boosted trees | real"):
            b = cl["arms"][other]
            print("a1 vs", other, mcnemar(a["test_real"]["pred"], b["test_real"]["pred"], gold),
                  "loso", a["loso"]["k"], "vs", b["loso"]["k"])
            for name in ("encoder-ft-real", "lm135-lora-real"):
                if load(name):
                    print(name, "vs", other, mcnemar(load(name)["test_real"]["pred"], b["test_real"]["pred"], gold))
        if hb and load("encoder-ft-real"):
            h = hb["arms"]["c3 habit table | real | label"]["test_real"]["pred"]
            print("encoder-ft-real vs habit table", mcnemar(load("encoder-ft-real")["test_real"]["pred"], h, gold))
            print("a1 vs habit table", mcnemar(a["test_real"]["pred"], h, gold))
    wf = load("wake-filter")
    if wf:
        print(f"\nwake filter, test_real ({wf['waits']['test_real']} waits in 108)\n| arm | threshold from training day | "
              "filtered | precision % | share of all decisions % | best possible on the test day itself |\n|---|---|---|---|---|---|")
        for k, v in wf["arms"].items():
            t, o = v["test_real"]["train_threshold"], v["test_real"]["oracle"]
            print(f"| {k} | {v['t95_train']} | {t['filtered'] if t else 'none exists'} | {t['precision'] if t else '-'} | "
                  f"{t['coverage'] if t else 0} | {o['filtered']} at {o['precision']} % ({o['coverage']} %) |")
    for name in ("latency-classic", "latency-lm"):
        la = load(name)
        if la:
            print(f"\n{name} (threads {la['threads']}, mean tokens {la['tokens_mean']})\n| path | p50 ms | p90 ms | n | "
                  "gate | foreign cores | compiler seen |\n|---|---|---|---|---|---|---|")
            for k, v in la["rows"].items():
                print(f"| {k} | {v['p50']} | {v['p90']} | {v['n']} | {v['idle']['gate']} | {v['during']['foreign_cores_mean']} "
                      f"| {'yes' if v['contaminated'] else 'no'} |")


if __name__ == "__main__":
    main()
