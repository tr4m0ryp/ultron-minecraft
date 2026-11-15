"""Arms (a) to (c): trained heads on encoder embeddings, nearest neighbour, and the feature classifier.

  python classic.py --out ../results/trained/classic.json

Training sets: real (2026-11-14 sessions), real+vary, real+vary+rules. Test sets: test_real (2026-11-15
sessions) and test_hand (cases_nochat.jsonl). 'loso' refits on all real pairs but one session and predicts that
session, over all 11 sessions: a second view with more cases; thresholds are never fitted there.
The 95 % precision threshold of each arm is chosen on predictions for the training sessions (leave one training
session out), then applied unchanged to the test sets.
"""
import argparse, json, os, sys, warnings
import numpy as np
from sklearn.ensemble import HistGradientBoostingClassifier
from sklearn.feature_extraction import DictVectorizer
from sklearn.linear_model import LogisticRegression
from sklearn.neural_network import MLPClassifier
from sklearn.pipeline import make_pipeline
from sklearn.preprocessing import MaxAbsScaler
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import data, metrics
from features import features

warnings.filterwarnings("ignore")


class KNN:
    """Cosine nearest neighbours; confidence is the similarity-weighted vote share times the top similarity."""

    def __init__(self, k):
        self.k = k

    def fit(self, X, y):
        self.X, self.y = X, np.array(y); self.classes_ = sorted(set(y)); return self

    def predict_proba(self, X):
        sim = X @ self.X.T
        top = np.argsort(-sim, axis=1)[:, :self.k]
        P = np.zeros((len(X), len(self.classes_)))
        for i, idx in enumerate(top):
            w = np.maximum(sim[i, idx], 1e-6)
            for j, wj in zip(idx, w):
                P[i, self.classes_.index(self.y[j])] += wj
            P[i] = P[i] / P[i].sum() * sim[i, idx[0]]
        return P


def arms():
    lr = lambda C: LogisticRegression(C=C, max_iter=3000)
    return {
        "a1 encoder + logistic head": ("emb", lambda: lr(10.0)),
        "a2 encoder + MLP head": ("emb", lambda: MLPClassifier((256,), max_iter=150, tol=1e-3, random_state=0)),
        "a3 encoder + features + logistic head": ("both", lambda: lr(10.0)),
        "b1 encoder nearest neighbour k=1": ("emb", lambda: KNN(1)),
        "b5 encoder nearest neighbours k=5": ("emb", lambda: KNN(5)),
        "c1 features + logistic regression": ("feat", lambda: make_pipeline(MaxAbsScaler(), lr(1.0))),
        "c2 features + boosted trees": ("feat", lambda: HistGradientBoostingClassifier(max_iter=80, learning_rate=0.1,
                                                                                      max_depth=4, random_state=0)),
    }


class View:
    """Builds the input matrix of one kind (emb, feat, both) for any list of pairs; vectoriser fitted on train."""

    def __init__(self, kind, enc):
        self.kind, self.enc, self.dv, self.sc = kind, enc, DictVectorizer(sparse=False), MaxAbsScaler()

    def fit(self, pairs):
        if self.kind != "emb":
            self.sc.fit(self.dv.fit_transform([features(p) for p in pairs]))
        return self.x(pairs)

    def x(self, pairs):
        if self.kind == "emb":
            return self.enc.many(pairs)
        F = self.dv.transform([features(p) for p in pairs])
        return F if self.kind == "feat" else np.hstack([self.enc.many(pairs), self.sc.transform(F)])


def fit_predict(kind, make, train, tests, enc, label):
    v = View(kind, enc)
    y = [label(p) for p in train]
    if len(set(y)) < 2:
        return [([y[0]] * len(t), [1.0] * len(t)) for t in tests]
    m = make().fit(v.fit(train), y)
    classes = m.classes_
    out = []
    for t in tests:
        P = m.predict_proba(v.x(t))
        out.append(([str(classes[i]) for i in P.argmax(1)], [float(c) for c in P.max(1)]))
    return out


def by_session(pairs):
    s = {}
    for p in pairs:
        s.setdefault(p["session"], []).append(p)
    return s


def held_out_predictions(kind, make, pairs, extra, enc, label):
    """Leave one session out over `pairs` (+extra synthetic rows in every fit). -> pred, conf, gold in order."""
    pred, conf, gold = [], [], []
    for name, test in by_session(pairs).items():
        train = [p for p in pairs if p["session"] != name] + [e for e in extra if e.get("src_session") != name]
        (pr, cf), = fit_predict(kind, make, train, [test], enc, label)
        pred += pr; conf += cf; gold += [label(p) for p in test]
    return pred, conf, gold


def score(pred, conf, gold, t95):
    return {**metrics.acc(pred, gold), "curve": metrics.curve(pred, conf, gold),
            "cov95_oracle": metrics.coverage_at(pred, conf, gold),
            "cov95_train_threshold": metrics.at_threshold(pred, conf, gold, t95), "pred": pred,
            "conf": [round(c, 3) for c in conf]}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=os.path.join(data.ROOT, "results", "trained", "classic.json"))
    ap.add_argument("--label", choices=("label", "detail"), default="label")
    ap.add_argument("--no-loso", action="store_true")
    ap.add_argument("--encoder", choices=("bge", "lm135"), default="bge", help="lm135: frozen 135M model as encoder")
    ap.add_argument("--arms", default="", help="comma-separated arm prefixes, e.g. a1,b5 (default: all)")
    ap.add_argument("--sets", default="real,real+vary,real+vary+rules")
    a = ap.parse_args()
    from embed import Encoder, LMEncoder
    enc = Encoder() if a.encoder == "bge" else LMEncoder()
    label = (lambda p: p["label"]) if a.label == "label" else data.detail_label
    train, test_real = data.real()
    hand, vary, rules = data.hand(), data.synth("vary"), data.synth("rules")
    for p in vary:
        p["src_session"] = p["session"]
    sets = {"real": train, "real+vary": train + vary, "real+vary+rules": train + vary + rules}
    extra = {"real": [], "real+vary": vary, "real+vary+rules": vary + rules}
    res = {"label": a.label, "n": {"train_real": len(train), "vary": len(vary), "rules": len(rules),
                                   "test_real": len(test_real), "test_hand": len(hand)}, "arms": {}}
    for name, (kind, make) in arms().items():
        if a.arms and name.split()[0] not in a.arms.split(","):
            continue
        for sname, tr in sets.items():
            if (a.label == "detail" and sname != "real") or sname not in a.sets.split(","):
                continue
            cp, cc, cg = held_out_predictions(kind, make, train, extra[sname], enc, label)
            t95 = metrics.threshold_for(cp, cc, cg)
            (pr, cr), (ph, ch) = fit_predict(kind, make, tr, [test_real, hand], enc, label)
            row = {"t95": t95, "train_cv": metrics.acc(cp, cg),
                   "test_real": score(pr, cr, [label(p) for p in test_real], t95),
                   "test_hand": score(ph, ch, [label(p) for p in hand], t95)}
            if sname == "real" and not a.no_loso:
                lp, lc, lg = held_out_predictions(kind, make, train + test_real, [], enc, label)
                row["loso"] = score(lp, lc, lg, None)
            res["arms"][f"{name} | {sname}"] = row
            print(f"{name:42s} {sname:16s} cv {row['train_cv']['k']}/{row['train_cv']['n']} real "
                  f"{row['test_real']['k']}/108 hand {row['test_hand']['k']}/48 t95 {t95} "
                  f"cov {row['test_real']['cov95_train_threshold']['coverage']} "
                  f"prec {row['test_real']['cov95_train_threshold']['precision']}"
                  + (f" loso {row['loso']['k']}/{row['loso']['n']}" if "loso" in row else ""), flush=True)
    os.makedirs(os.path.dirname(a.out), exist_ok=True)
    json.dump(res, open(a.out, "w"), indent=1)


if __name__ == "__main__":
    main()
