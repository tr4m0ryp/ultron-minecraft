"""Benchmark a System-One style decider on bench/decider/cases.jsonl (60 Minecraft cases).

  python run.py --backend von --label von --out results/von.json
  python run.py --backend http --url http://127.0.0.1:8009 --model kev-latest --label kev-0.8b --pid <server pid>
Measures: accuracy, p50/p95 per single-question call (>=200 after warm-up), 5-questions-in-one-call latency, RSS.
"""
import argparse, json, os, resource, statistics, sys, time
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
t_proc0 = time.perf_counter()
import psutil
from backends import BACKENDS

def load_cases():
    return [json.loads(l) for l in open(os.path.join(HERE, "cases.jsonl"))]

def decide(case, ans):
    k, a = case["kind"], ans["q"]
    if k in ("choice", "rank"):
        return a["choice"]
    if k == "yesno":
        return "yes" if a["noul"] > 0.5 else "no"
    return max(0, min(len(case["question"]["criteria"]) - 1, int(round(a["score"]))))

def pct(xs, p):
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(round(p / 100 * (len(xs) - 1))))]

def ms(xs):
    return {"p50": round(pct(xs, 50) * 1e3, 1), "p95": round(pct(xs, 95) * 1e3, 1), "mean": round(statistics.mean(xs) * 1e3, 1), "n": len(xs)}

def rss(pid):
    p = psutil.Process(pid)
    return sum(c.memory_info().rss for c in [p] + p.children(recursive=True)) / 1e9

def five_questions(cases, i):
    """Five typed questions about ONE state: the case's own plus four generic ones."""
    c = cases[i]
    qs = {"q": c["question"],
          "danger": {"type": "noul", "instructions": "Is the player in immediate danger?"},
          "interrupt": {"type": "noul", "instructions": "Should the player interrupt the current goal right now?"},
          "threat": {"type": "score", "instructions": "How threatening is the situation?", "criteria": ["Safe", "Risky", "Deadly"]},
          "act": {"type": "choice", "instructions": "What should the player do right now?",
                  "criteria": {"fight": "Fight a mob", "flee": "Move away", "continue": "Continue the current task", "eat": "Eat food"}}}
    return c["state"], qs

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--backend", required=True, choices=list(BACKENDS))
    ap.add_argument("--label", required=True)
    ap.add_argument("--out", default=None)
    ap.add_argument("--n-lat", type=int, default=240)
    ap.add_argument("--n-batch", type=int, default=100)
    ap.add_argument("--stride", type=int, default=1, help="use every Nth case (bounds runtime on slow CPUs)")
    ap.add_argument("--threads", type=int, default=8)
    ap.add_argument("--pid", type=int, default=None, help="server pid for RSS (http backend)")
    ap.add_argument("--load-kw", default="{}", help="json kwargs for backend.load")
    a = ap.parse_args()
    os.environ.setdefault("OMP_NUM_THREADS", str(a.threads))
    cases = load_cases()[::a.stride]
    be = BACKENDS[a.backend]()
    if a.backend != "http":
        import torch
        torch.set_num_threads(a.threads)
    t0 = time.perf_counter()
    be.load(**json.loads(a.load_kw))
    t_load = time.perf_counter() - t0
    t0 = time.perf_counter()
    be.predict(cases[0]["state"], {"q": cases[0]["question"]})
    t_first = time.perf_counter() - t0
    for i in range(min(3, len(cases))):  # warm-up
        be.predict(cases[i]["state"], {"q": cases[i]["question"]})
    # accuracy + latency in the same pass structure: cycle all 60 cases, repeat until n-lat calls
    correct, per_kind, lat, errs = 0, {}, [], []
    n_pass = max(1, -(-a.n_lat // len(cases)))
    for p in range(n_pass):
        for c in cases:
            st = c["state"] if p == 0 else f"{c['state']} | tick {p * 1000 + len(lat)}"  # unique state: defeats exact-state caches
            t = time.perf_counter()
            try:
                ans = be.predict(st, {"q": c["question"]})
            except Exception as e:
                errs.append(f"{c['id']}: {type(e).__name__}: {e}"[:200]); continue
            lat.append(time.perf_counter() - t)
            if len(lat) % 10 == 0: print(f'[{a.label}] {len(lat)} calls, last {lat[-1]*1e3:.0f} ms', file=sys.stderr, flush=True)
            if p == 0:
                ok = decide(c, ans) == c["answer"]
                correct += ok
                kk = per_kind.setdefault(c["kind"], [0, 0]); kk[0] += ok; kk[1] += 1
    blat = []
    for j in range(a.n_batch):
        st, qs = five_questions(cases, j % len(cases))
        st = f"{st} | tick {90000 + j}"
        t = time.perf_counter()
        try:
            be.predict(st, qs)
            blat.append(time.perf_counter() - t)
        except Exception as e:
            errs.append(f"batch5: {type(e).__name__}: {e}"[:200]); break
    pid = a.pid or os.getpid()
    res = {"label": a.label, "backend": a.backend, "threads": a.threads,
           "accuracy": round(correct / len(cases), 3), "correct": correct, "n_cases": len(cases),
           "by_kind": {k: f"{v[0]}/{v[1]}" for k, v in per_kind.items()},
           "single_ms": ms(lat) if lat else None, "batch5_ms": ms(blat) if blat else None,
           "load_s": round(t_load, 2), "first_call_s": round(t_first, 2),
           "rss_gb": round(rss(pid), 2), "peak_rss_gb": round(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1e6, 2) if not a.pid else None,
           "errors": errs[:5], "n_errors": len(errs)}
    print(json.dumps(res, indent=1))
    if a.out:
        os.makedirs(os.path.dirname(os.path.abspath(a.out)), exist_ok=True)
        json.dump(res, open(a.out, "w"), indent=1)

if __name__ == "__main__":
    main()
