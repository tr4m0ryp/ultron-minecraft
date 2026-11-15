"""Quantized small models through llama.cpp, deciding by argmax over option logits.

  python logit_run.py --model ~/models/intuition/gguf/X.gguf --label x-q4 --threads 4 --reps 3 --out results/x.json

Prompt layout puts everything that does not change per decision first (task line, question, options) and the
game state last, so the fixed part can stay in the KV cache. Per case three timings are taken:
  cold     reset, evaluate the whole prompt, read the logits of the answer position (one forward pass)
  cached   keep the prefix in the KV cache, evaluate only the state suffix, read the same logits
  generate (--generate) cold prompt, then sample the answer greedily as text and parse it
Options are labelled A, B, C ... and shuffled per case with a fixed seed, because in cases.jsonl the right
answer is the first option in most choice cases. --calib subtracts the logits of a content-free state.
"""
import argparse, json, os, random, statistics, sys, time
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import numpy as np
from idle import Monitor, gate
from trivial import options

HEAD = "You are the fast instinct of a Minecraft player. Read the state and pick the single best option.\n"
LETTERS = "ABCDEFGHIJKLMNOP"


def build(case, shuffle):
    q, opts = case["question"], options(case)
    if q["type"] == "noul":
        desc = {"yes": "Yes", "no": "No"}
    elif q["type"] == "score":
        desc = dict(enumerate(q["criteria"]))
    else:
        desc = q["criteria"]
    order = list(opts)
    if shuffle:
        random.Random(case["id"]).shuffle(order)
    lines = "".join(f"{LETTERS[i]}. {desc[o]}\n" if q["type"] != "choice" else f"{LETTERS[i]}. {o}: {desc[o]}\n"
                    for i, o in enumerate(order))
    prefix = f"{HEAD}Question: {q['instructions']}\nOptions:\n{lines}State:"
    return prefix, " {}\nAnswer:", order


def pct(xs, p):
    xs = sorted(xs)
    return round(xs[min(len(xs) - 1, int(round(p / 100 * (len(xs) - 1))))] * 1e3, 1)


def stats(xs):
    return {"p50": pct(xs, 50), "p90": pct(xs, 90), "mean": round(statistics.mean(xs) * 1e3, 1), "n": len(xs)}


class Scorer:
    def __init__(self, path, threads):
        from llama_cpp import Llama
        self.llm = Llama(model_path=os.path.expanduser(path), n_ctx=1024, n_batch=1024, n_threads=threads,
                         n_threads_batch=threads, logits_all=False, verbose=False)
        self.letter = [self._tok1(" " + l) for l in LETTERS]

    def _tok1(self, text):
        return self.llm.tokenize(text.encode(), add_bos=False, special=False)[0]

    def tok(self, text, bos):
        return self.llm.tokenize(text.encode(), add_bos=bos, special=False)

    def rewind(self, n):
        self.llm._ctx.kv_cache_seq_rm(-1, n, -1)
        self.llm.n_tokens = n

    def logits(self, k):
        row = np.array(self.llm.scores[self.llm.n_tokens - 1], dtype=np.float32)
        return row[self.letter[:k]]

    def generate(self, k, max_new=4):
        """Greedy text answer after the prompt already in context. -> option index or None."""
        out = b""
        for _ in range(max_new):
            t = int(np.argmax(self.llm.scores[self.llm.n_tokens - 1]))
            if t == self.llm.token_eos():
                break
            out += self.llm.detokenize([t])
            s = out.decode(errors="ignore").strip()
            if s[:1] in LETTERS[:k] and s[:1]:
                return LETTERS.index(s[0])
            self.llm.eval([t])
        return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", required=True); ap.add_argument("--label", required=True)
    ap.add_argument("--cases", default=os.path.join(HERE, "cases.jsonl"))
    ap.add_argument("--stride", type=int, default=1); ap.add_argument("--threads", type=int, default=4)
    ap.add_argument("--reps", type=int, default=3); ap.add_argument("--out", default=None)
    ap.add_argument("--generate", action="store_true"); ap.add_argument("--calib", action="store_true")
    ap.add_argument("--no-shuffle", action="store_true"); ap.add_argument("--gate-wait", type=int, default=300)
    a = ap.parse_args()
    cases = [json.loads(l) for l in open(a.cases)][::a.stride]
    sc = Scorer(a.model, a.threads)
    built = []
    for c in cases:
        prefix, suffix, order = build(c, not a.no_shuffle)
        built.append((sc.tok(prefix, True), sc.tok(suffix.format(c["state"]), False), sc.tok(suffix.format("unknown"), False), order))
    sc.llm.reset(); sc.llm.eval(built[0][0] + built[0][1])  # warm-up
    idle = gate(wait_s=a.gate_wait)
    cold, cached, gen, runs = [], [], [], []
    with Monitor() as mon:
        for r in range(a.reps):
            ok = {"argmax": 0, "calib": 0, "generate": 0}
            for c, (pre, suf, null, order) in zip(cases, built):
                k, gold = len(order), order.index(c["answer"])
                t = time.perf_counter(); sc.llm.reset(); sc.llm.eval(pre + suf); lg = sc.logits(k); cold.append(time.perf_counter() - t)
                ok["argmax"] += int(np.argmax(lg)) == gold
                if a.generate:
                    sc.llm.reset()
                    t = time.perf_counter(); sc.llm.eval(pre + suf); g = sc.generate(k); gen.append(time.perf_counter() - t)
                    ok["generate"] += g == gold
                sc.llm.reset(); sc.llm.eval(pre)  # the fixed part, paid once per question, not per decision
                if a.calib:
                    sc.llm.eval(null); base = sc.logits(k); sc.rewind(len(pre))
                t = time.perf_counter(); sc.rewind(len(pre)); sc.llm.eval(suf); lg2 = sc.logits(k); cached.append(time.perf_counter() - t)
                assert int(np.argmax(lg2)) == int(np.argmax(lg)) or abs(float(np.max(lg2) - np.max(lg))) < 0.5
                if a.calib:
                    ok["calib"] += int(np.argmax(lg2 - base)) == gold
            runs.append(ok)
    n = len(cases)
    res = {"label": a.label, "model": os.path.basename(a.model), "cases": os.path.basename(a.cases), "n_cases": n,
           "threads": a.threads, "reps": a.reps, "shuffled": not a.no_shuffle,
           "correct_argmax": runs[0]["argmax"], "correct_calib": runs[0]["calib"] if a.calib else None,
           "correct_generate": runs[0]["generate"] if a.generate else None,
           "deterministic": all(r == runs[0] for r in runs),
           "prompt_tokens_mean": round(statistics.mean(len(p) + len(s) for p, s, _, _ in built), 1),
           "suffix_tokens_mean": round(statistics.mean(len(s) for _, s, _, _ in built), 1),
           "cold_ms": stats(cold), "cached_ms": stats(cached), "generate_ms": stats(gen) if gen else None,
           "idle": idle, "during": mon.summary()}
    print(json.dumps(res))
    if a.out:
        os.makedirs(os.path.dirname(os.path.abspath(a.out)), exist_ok=True)
        json.dump(res, open(a.out, "w"), indent=1)


if __name__ == "__main__":
    main()
