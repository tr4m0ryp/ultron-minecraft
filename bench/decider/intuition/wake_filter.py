"""The one take-over that needs no parameters: 'Thought would only wait here, do not wake it'.

  python wake_filter.py --out ../results/trained/wake-filter.json

Binary target wait / act for the encoder head, the feature classifier and the one-line rule (goal_blocked means
wait). A wake-up is filtered when P(wait) >= t. Reported per threshold: filtered wake-ups, how many of them
Thought really answered with wait (precision), the share of all decisions that is (coverage), and the share of
all waits caught (recall). The threshold marked 'train' is the lowest one with precision >= 95 % on predictions
for the training sessions (leave one session out); the 'oracle' one is chosen on the test set itself.
"""
import argparse, json, os, sys, warnings
import numpy as np
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import data
from classic import View, arms, by_session
from features import event_parts

warnings.filterwarnings("ignore")
ARMS = ("a1 encoder + logistic head", "c1 features + logistic regression", "c2 features + boosted trees")
WAIT = lambda p: "wait" if p["label"] == "wait" else "act"


def p_wait(kind, make, train, test, enc):
    v = View(kind, enc)
    m = make().fit(v.fit(train), [WAIT(p) for p in train])
    return m.predict_proba(v.x(test))[:, list(m.classes_).index("wait")]


def at(pw, gold, t):
    idx = [i for i, c in enumerate(pw) if c >= t]
    k, waits = sum(gold[i] == "wait" for i in idx), sum(g == "wait" for g in gold)
    return {"t": round(float(t), 3), "filtered": len(idx), "correct": k,
            "precision": round(100 * k / len(idx), 1) if idx else None,
            "coverage": round(100 * len(idx) / len(gold), 1), "recall": round(100 * k / waits, 1) if waits else None}


def best_t(pw, gold, precision=0.95):
    """Lowest threshold whose filtered set keeps the precision; None when no threshold does."""
    ok = [t for t in sorted(set(pw)) if (a := at(pw, gold, t))["filtered"] and a["precision"] >= 100 * precision]
    return float(ok[0]) if ok else None


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--out", required=True)
    a = ap.parse_args()
    from embed import Encoder
    enc = Encoder()
    train, test = data.real()
    hand = data.hand()
    res = {"waits": {"train": sum(WAIT(p) == "wait" for p in train), "test_real": sum(WAIT(p) == "wait" for p in test),
                     "test_hand": sum(WAIT(p) == "wait" for p in hand)}, "arms": {}}
    rule = lambda ps: np.array([float(event_parts(p["event"])[0] == "goal_blocked") for p in ps])
    for name in ARMS + ("rule: goal_blocked means wait",):
        if name.startswith("rule"):
            cv, cg = rule(train), [WAIT(p) for p in train]
            pr, ph = rule(test), rule(hand)
        else:
            kind, make = arms()[name]
            cv, cg = [], []
            for s, held in by_session(train).items():
                cv += list(p_wait(kind, make, [p for p in train if p["session"] != s], held, enc)); cg += [WAIT(p) for p in held]
            pr, ph = p_wait(kind, make, train, test, enc), p_wait(kind, make, train, hand, enc)
        t = best_t(cv, cg)
        row = {"train_cv_at_0.5": at(cv, cg, 0.5), "t95_train": t}
        for sname, pw, ps in (("test_real", pr, test), ("test_hand", ph, hand)):
            gold = [WAIT(p) for p in ps]
            to = best_t(list(pw), gold)
            row[sname] = {"curve": [at(pw, gold, x) for x in (0.5, 0.7, 0.8, 0.9, 0.95)],
                          "train_threshold": at(pw, gold, t) if t is not None else None,
                          "oracle": at(pw, gold, to) if to is not None else None}
        res["arms"][name] = row
        print(name, "t95", t, "| real", row["test_real"]["train_threshold"], "| oracle", row["test_real"]["oracle"],
              "| hand", row["test_hand"]["train_threshold"], flush=True)
    json.dump(res, open(a.out, "w"), indent=1)


if __name__ == "__main__":
    main()
