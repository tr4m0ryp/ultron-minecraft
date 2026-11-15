"""The cheap arms on the same 3 session folds as tune_encoder.py --folds 3, for a paired comparison.

  python folds.py --out ../results/trained/folds3.json
"""
import argparse, json, os, sys, warnings
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import baselines, data, habit
from classic import arms, fit_predict, score
from tune_encoder import folds

warnings.filterwarnings("ignore")
ARMS = ("a1 encoder + logistic head", "a2 encoder + MLP head", "b5 encoder nearest neighbours k=5",
        "c1 features + logistic regression", "c2 features + boosted trees")


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--out", required=True)
    a = ap.parse_args()
    from embed import Encoder
    enc = Encoder()
    tr, te = data.real()
    label = lambda p: p["label"]
    out = {n: ([], []) for n in ARMS + ("c3 habit table", "repeat the last decision")}
    gold, sess = [], []
    for train, test in folds(tr + te, 3):
        gold += [label(p) for p in test]; sess += [(p["session"], p["i"]) for p in test]
        for n in ARMS:
            (pr, cf), = fit_predict(*arms()[n], train, [test], enc, label)
            out[n][0].extend(pr); out[n][1].extend(cf)
        (pr, cf), = habit.run(train, [test], label)
        out["c3 habit table"][0].extend(pr); out["c3 habit table"][1].extend(cf)
        out["repeat the last decision"][0].extend(baselines.repeat_last(p) for p in test)
        out["repeat the last decision"][1].extend(1.0 for _ in test)
    res = {"cases": sess, "gold": gold, "arms": {n: score(p, c, gold, None) for n, (p, c) in out.items()}}
    for n, v in res["arms"].items():
        print(n, v["k"], "/", v["n"], v["ci95"], v["cov95_oracle"])
    json.dump(res, open(a.out, "w"), indent=1)


if __name__ == "__main__":
    main()
