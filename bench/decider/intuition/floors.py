"""Floors on the new test sets: chance, majority, repeat the last decision, yesterday's keyword rule, the
one-line rule 'goal_blocked means wait', and the encoder with the nearest of 12 solved examples (untrained).

  python floors.py --out ../results/trained/floors.json
"""
import argparse, json, os, sys
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import baselines, data, metrics
from classic import KNN, score
from features import event_parts
from lm_cases import OPTIONS


def blocked_rule(p):
    """Read off the training day: while the body is re-planning a blocked goal, Thought waits."""
    return ("wait", 1.0) if event_parts(p["event"])[0] == "goal_blocked" else ("wake", 0.0)


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--out", required=True)
    a = ap.parse_args()
    from embed import Encoder
    enc = Encoder()
    train, test_real = data.real()
    sets = {"test_real": test_real, "test_hand": data.hand(), "all_real": train + test_real, "train": train}
    seen, shots = set(), []
    for p in train:
        if p["label"] not in seen:
            seen.add(p["label"]); shots.append(p)
    nn = KNN(1).fit(enc.many(shots), [p["label"] for p in shots])
    maj = baselines.majority(train)
    res = {"chance_options": len(OPTIONS), "majority_label": maj, "rows": {}}
    for name, ps in sets.items():
        gold = [p["label"] for p in ps]
        P = nn.predict_proba(enc.many(ps))
        rule = [blocked_rule(p) for p in ps]
        res["rows"][name] = {
            "chance": round(len(ps) / len(OPTIONS), 1),
            "majority": metrics.acc([maj] * len(ps), gold),
            "repeat_last": metrics.acc([baselines.repeat_last(p) for p in ps], gold),
            "keyword_rule": metrics.acc([baselines.keyword(p) for p in ps], gold),
            "encoder_nearest_of_12": score([nn.classes_[i] for i in P.argmax(1)], [float(c) for c in P.max(1)], gold, None),
            "blocked_means_wait": metrics.at_threshold([r[0] for r in rule], [r[1] for r in rule], gold, 0.5),
            "wait_share": metrics.acc(["wait"] * len(ps), gold)}
        print(name, {k: (v if not isinstance(v, dict) else (v.get("k", v.get("correct")), v.get("n", v.get("taken"))))
                     for k, v in res["rows"][name].items()})
    json.dump(res, open(a.out, "w"), indent=1)


if __name__ == "__main__":
    main()
