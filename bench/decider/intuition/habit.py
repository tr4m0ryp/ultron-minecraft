"""Arm (c3): a habit table. No model at all: count what Thought did after each kind of event and do the same.

  python habit.py --out ../results/trained/habit.json

Keys from specific to general; the first key seen at least MIN times answers:
  (event kind, event goal, failure reason, same item as last decision, last decision)
  (event kind, event goal, failure reason)      (event kind, event goal)      (event kind)      ()
Confidence is the smoothed share of the winning decision under that key: (wins + 1) / (seen + 2).
"""
import argparse, collections, json, os, re, sys
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import data, metrics
from classic import by_session, score
from features import REASONS, event_parts

MIN = 2


def keys(p):
    kind, goal, item = event_parts(p["event"])
    why = next((n for n, pat in REASONS.items() if re.search(pat, p["event"])), "")
    last = p["history"][-1].split() if p["history"] else ["none"]
    same = bool(item) and item in last
    running = bool(re.search(r"; goal (?:#\d+ )?\w+ .*running", p["snapshot"]))
    return [(kind, goal, why, same, last[0], running), (kind, goal, why, running), (kind, goal), (kind,), ()]


class Habit:
    def fit(self, pairs, label):
        self.t = collections.defaultdict(collections.Counter)
        for p in pairs:
            for k in keys(p):
                self.t[k][label(p)] += 1
        return self

    def predict(self, p):
        for k in keys(p):
            c = self.t.get(k)
            if c and sum(c.values()) >= MIN:
                lab, n = c.most_common(1)[0]
                return lab, (n + 1) / (sum(c.values()) + 2)
        return "wait", 0.0


def run(train, tests, label):
    h = Habit().fit(train, label)
    return [tuple(zip(*[h.predict(p) for p in t])) for t in tests]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=os.path.join(data.ROOT, "results", "trained", "habit.json"))
    a = ap.parse_args()
    train, test_real = data.real()
    hand, rules = data.hand(), data.synth("rules")
    res = {"arms": {}}
    for lname, label in (("label", lambda p: p["label"]), ("detail", data.detail_label)):
        for sname, extra in (("real", []), ("real+rules", rules)):
            cp, cc, cg = [], [], []
            for s, test in by_session(train).items():
                (pr, cf), = run([p for p in train if p["session"] != s] + extra, [test], label)
                cp += pr; cc += cf; cg += [label(p) for p in test]
            t95 = metrics.threshold_for(cp, cc, cg)
            (pr, cr), (ph, ch) = run(train + extra, [test_real, hand], label)
            lp, lc, lg = [], [], []
            for s, test in by_session(train + test_real).items():
                (p1, c1), = run([p for p in train + test_real if p["session"] != s] + extra, [test], label)
                lp += p1; lc += c1; lg += [label(p) for p in test]
            row = {"t95": t95, "train_cv": metrics.acc(cp, cg),
                   "test_real": score(list(pr), list(cr), [label(p) for p in test_real], t95),
                   "test_hand": score(list(ph), list(ch), [label(p) for p in hand], t95),
                   "loso": score(lp, lc, lg, None)}
            res["arms"][f"c3 habit table | {sname} | {lname}"] = row
            print(lname, sname, "cv", row["train_cv"]["k"], "real", row["test_real"]["k"], "hand", row["test_hand"]["k"],
                  "t95", t95, row["test_real"]["cov95_train_threshold"], "loso", row["loso"]["k"], row["loso"]["cov95_oracle"])
    json.dump(res, open(a.out, "w"), indent=1)


if __name__ == "__main__":
    main()
