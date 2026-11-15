"""Accuracy with a 95 % Wilson interval, and accuracy against coverage when acting only above a confidence."""
import math

THRESHOLDS = (0.0, 0.5, 0.6, 0.7, 0.8, 0.9, 0.95)


def wilson(k, n, z=1.96):
    if n == 0:
        return (0.0, 0.0)
    p, d = k / n, 1 + z * z / n
    c, h = (p + z * z / (2 * n)) / d, z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
    return (round(100 * (c - h), 1), round(100 * (c + h), 1))


def acc(pred, gold):
    k = sum(a == b for a, b in zip(pred, gold))
    return {"k": k, "n": len(gold), "ci95": wilson(k, len(gold))}


def curve(pred, conf, gold, thresholds=THRESHOLDS):
    """-> rows {t, taken, coverage %, correct, precision %} for 'act when confidence >= t, else wake Thought'."""
    rows = []
    for t in thresholds:
        idx = [i for i, c in enumerate(conf) if c >= t]
        k = sum(pred[i] == gold[i] for i in idx)
        rows.append({"t": t, "taken": len(idx), "coverage": round(100 * len(idx) / len(gold), 1), "correct": k,
                     "precision": round(100 * k / len(idx), 1) if idx else None})
    return rows


def coverage_at(pred, conf, gold, precision=0.95):
    """Largest share of decisions that can be taken with precision >= target, threshold chosen on these same
    cases (an upper bound: a deployed threshold has to be chosen on other data)."""
    order = sorted(range(len(gold)), key=lambda i: -conf[i])
    best, k = {"taken": 0, "coverage": 0.0, "t": None, "precision": None}, 0
    for n, i in enumerate(order, 1):
        k += pred[i] == gold[i]
        last_of_tie = n == len(order) or conf[order[n]] < conf[i]
        if last_of_tie and k / n >= precision:
            best = {"taken": n, "coverage": round(100 * n / len(gold), 1), "t": round(float(conf[i]), 3),
                    "precision": round(100 * k / n, 1)}
    return best


def threshold_for(pred, conf, gold, precision=0.95):
    """Lowest confidence threshold that keeps precision >= target on (cross-validated) training predictions."""
    return coverage_at(pred, conf, gold, precision)["t"]


def at_threshold(pred, conf, gold, t):
    if t is None:
        return {"t": None, "taken": 0, "coverage": 0.0, "correct": 0, "precision": None}
    return curve(pred, conf, gold, (t,))[0]
