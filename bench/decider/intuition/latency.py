"""Per-decision latency of the trained arms, one decision at a time, behind the idle gate.

  EPY latency.py classic --out ../results/trained/latency-classic.json     (torch + scikit-learn environment)
  PY  latency.py lm --model ~/models/intuition/gguf/SmolLM2-135M-Instruct-f16.gguf --out ...   (llama.cpp one)

classic: the whole path from the pair to a label: render text, tokenise, encode, head; or parse features, tree
or linear model. lm: the adapter prompt (pair text + 'Answer:', nothing cached) through llama.cpp on the base
weights. A merged adapter changes no shapes, so its cost per token is the same; accuracy comes from tune_lm.py.
A run during which a compiler was seen, or that started with one running, is marked contaminated.
"""
import argparse, json, os, sys, time
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE); sys.path.insert(0, os.path.dirname(HERE))
import data
from idle import Monitor, gate


def pct(xs, p):
    xs = sorted(xs)
    return round(xs[min(len(xs) - 1, int(round(p / 100 * (len(xs) - 1))))] * 1e3, 2)


def timed(fn, cases, reps):
    fn(cases[0]); lat = []
    for _ in range(reps):
        for p in cases:
            t = time.perf_counter(); fn(p); lat.append(time.perf_counter() - t)
    return {"p50": pct(lat, 50), "p90": pct(lat, 90), "n": len(lat)}


def classic(a, cases):
    import torch
    from classic import View, arms
    from embed import Encoder
    torch.set_num_threads(a.threads)
    enc = Encoder(cache=False)
    train, _ = data.real()
    train = train + data.synth("vary") + data.synth("rules")
    y = [p["label"] for p in train]
    cached = Encoder()                     # training matrix from the disk cache; test texts are encoded live
    fns = {}
    for name, (kind, make) in arms().items():
        v = View(kind, cached)
        m = make().fit(v.fit(train), y)
        live = View(kind, enc); live.dv, live.sc = v.dv, v.sc
        fns[name] = lambda p, m=m, live=live: m.classes_[m.predict_proba(live.x([p])).argmax()]
    fns["encoder forward pass only"] = lambda p: enc.one(data.text(p))
    return fns, {"tokens_mean": round(sum(enc.tokens(data.text(p)) for p in cases) / len(cases), 1)}


def lm(a, cases):
    from logit_run import Scorer
    prompt = data.prompt
    sc = Scorer(a.model, a.threads)
    toks = {id(p): sc.tok(prompt(p), True) for p in cases}

    def run(p):
        sc.llm.reset(); sc.llm.eval(toks[id(p)]); return sc.row().argmax()
    return {"d 135M adapter prompt, llama.cpp, base weights": run}, \
        {"tokens_mean": round(sum(map(len, toks.values())) / len(toks), 1), "model": os.path.basename(a.model)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("what", choices=("classic", "lm")); ap.add_argument("--model", default=None)
    ap.add_argument("--threads", type=int, default=4); ap.add_argument("--reps", type=int, default=3)
    ap.add_argument("--gate-wait", type=int, default=300); ap.add_argument("--out", required=True)
    a = ap.parse_args()
    cases = data.real()[1] + data.hand()
    fns, info = (classic if a.what == "classic" else lm)(a, cases)
    res = {"threads": a.threads, **info, "rows": {}}
    for i, (name, fn) in enumerate(fns.items()):
        idle = gate(wait_s=a.gate_wait if i == 0 else 10)      # one full wait, then a short check per row
        with Monitor() as mon:
            ms = timed(fn, cases, a.reps)
        d = mon.summary()
        res["rows"][name] = {**ms, "idle": idle, "during": d,
                             "contaminated": bool(idle["compilers"] or d["compilers_seen"]),
                             "foreign_over_1_core": d["foreign_cores_mean"] > 1.0}
        print(name, ms, idle["gate"], d, flush=True)
    json.dump(res, open(a.out, "w"), indent=1)


if __name__ == "__main__":
    main()
