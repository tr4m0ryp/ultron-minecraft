"""Quantized small models through llama.cpp, deciding by argmax over option logits.

  python logit_run.py --model ~/models/intuition/gguf/X.gguf --label x-q4 --threads 4 --reps 3 --out results/x.json

Prompt layout puts everything that does not change per decision first (task line, question, options) and the
game state last, so the fixed part can stay in the KV cache. Per case three timings are taken:
  cold     reset, evaluate the whole prompt, read the logits of the answer position (one forward pass)
  cached   keep the prefix in the KV cache, evaluate only the state suffix, read the same logits
  generate (--generate) cold prompt, then sample the answer greedily as text and parse it
Options are labelled A, B, C ... and shuffled per case with a fixed seed, because in cases.jsonl the right
answer is the first option in most choice cases. --calib subtracts the logits of a content-free state.
--shots FILE puts solved examples in the cached prefix (they cost nothing per decision). --volatile moves the
state fields that rarely change (place, held, hotbar, time, goal) into the cached part, so only HP, hunger,
nearby, companion and chat are evaluated: the best case, when nothing else changed since the last decision.
--label key scores the first token of each option name instead of a letter.
"""
import argparse, json, os, random, statistics, sys, time
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import numpy as np
from idle import Monitor, gate
from trivial import options

HEAD = "You are the fast instinct of a Minecraft player. Read the state and pick the single best option.\n"
LETTERS = "ABCDEFGHIJKLMNOP"


STABLE = ("Place:", "Held:", "Hotbar:", "Time:", "Goal:")


def block(case, shuffle, by_key):
    """-> (question and options text, option order, answer text per option)."""
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
    named = q["type"] == "choice"
    by_key = by_key and named
    tag = [str(o) if by_key else LETTERS[i] for i, o in enumerate(order)]
    lines = "".join((f"- {o}: {desc[o]}\n" if by_key else f"{t}. {o}: {desc[o]}\n") if named else f"{t}. {desc[o]}\n"
                    for t, o in zip(tag, order))
    return f"Question: {q['instructions']}\nOptions:\n{lines}", order, tag


def split_state(state, volatile):
    if not volatile:
        return "", state
    fields = state.split(" | ")
    return " | ".join(f for f in fields if f.startswith(STABLE)) + " |", " | ".join(f for f in fields if not f.startswith(STABLE))


def build(case, shots, a):
    """-> (prefix text, suffix text, content-free suffix, option order, answer text per option)."""
    text, order, tag = block(case, not a.no_shuffle, a.label_by == "key")
    same_q = all(s["question"] == case["question"] for s in shots)
    ex = ""
    for s in shots:
        t2, o2, g2 = block(s, not a.no_shuffle, a.label_by == "key")
        ex += ("" if same_q else t2) + f"State: {s['state']}\nAnswer: {g2[o2.index(s['answer'])]}\n\n"
    stable, vol = split_state(case["state"], a.volatile)
    prefix = HEAD + (text + "\n" + ex if same_q else ex + text) + "State:" + (" " + stable if stable else "")
    return prefix, f" {vol}\nAnswer:", " unknown\nAnswer:", order, tag


def pct(xs, p):
    xs = sorted(xs)
    return round(xs[min(len(xs) - 1, int(round(p / 100 * (len(xs) - 1))))] * 1e3, 1)


def stats(xs):
    return {"p50": pct(xs, 50), "p90": pct(xs, 90), "mean": round(statistics.mean(xs) * 1e3, 1), "n": len(xs)}


class Scorer:
    def __init__(self, path, threads):
        from llama_cpp import Llama
        self.llm = Llama(model_path=os.path.expanduser(path), n_ctx=4096, n_batch=2048, n_threads=threads,
                         n_threads_batch=threads, logits_all=False, verbose=False)

    def first(self, tags):
        """First token of each answer text; None when two options share it."""
        ids = [self.llm.tokenize((" " + t).encode(), add_bos=False, special=False)[0] for t in tags]
        return ids if len(set(ids)) == len(ids) else None

    def tok(self, text, bos):
        return self.llm.tokenize(text.encode(), add_bos=bos, special=False)

    def rewind(self, n):
        self.llm._ctx.kv_cache_seq_rm(-1, n, -1)
        self.llm.n_tokens = n

    def row(self):
        """Logits of the last evaluated token, read from the context (the binding keeps none without logits_all)."""
        return np.ctypeslib.as_array(self.llm._ctx.get_logits(), shape=(self.llm.n_vocab(),))

    def logits(self, ids):
        return self.row()[ids].astype(np.float32)

    def generate(self, tags, max_new=6):
        """Greedy text answer after the prompt already in context, parsed afterwards. -> option index or None."""
        out = b""
        for _ in range(max_new):
            t = int(np.argmax(self.row()))
            if t == self.llm.token_eos():
                break
            out += self.llm.detokenize([t])
            self.llm.eval([t])
        words = out.decode(errors="ignore").strip().replace(".", " ").replace(":", " ").split()
        if os.environ.get("DEBUG_GEN"):
            print(repr(out), file=sys.stderr)
        return tags.index(words[0]) if words and words[0] in tags else None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", required=True); ap.add_argument("--label", required=True)
    ap.add_argument("--cases", default=os.path.join(HERE, "cases.jsonl"))
    ap.add_argument("--stride", type=int, default=1); ap.add_argument("--threads", type=int, default=4)
    ap.add_argument("--reps", type=int, default=3); ap.add_argument("--out", default=None)
    ap.add_argument("--generate", action="store_true"); ap.add_argument("--calib", action="store_true")
    ap.add_argument("--shots", default=None, help="jsonl of solved cases for the cached prefix")
    ap.add_argument("--n-shots", type=int, default=13); ap.add_argument("--volatile", action="store_true")
    ap.add_argument("--label-by", choices=("letter", "key"), default="letter")
    ap.add_argument("--no-shuffle", action="store_true"); ap.add_argument("--gate-wait", type=int, default=300)
    a = ap.parse_args()
    cases = [json.loads(l) for l in open(a.cases)][::a.stride]
    sc = Scorer(a.model, a.threads)
    shots = [json.loads(l) for l in open(a.shots)][:a.n_shots] if a.shots else []
    built = []
    for c in cases:
        prefix, suffix, null, order, tag = build(c, shots, a)
        ids = sc.first(tag)
        assert ids, f"{c['id']}: options share a first token"
        built.append((sc.tok(prefix, True), sc.tok(suffix, False), sc.tok(null, False), order, tag, ids))
    sc.llm.reset(); sc.llm.eval(built[0][0] + built[0][1])  # warm-up
    idle = gate(wait_s=a.gate_wait)
    cold, cached, gen, runs, wrong = [], [], [], [], []
    with Monitor() as mon:
        for r in range(a.reps):
            ok = {"argmax": 0, "calib": 0, "generate": 0}
            for c, (pre, suf, null, order, tag, ids) in zip(cases, built):
                gold = order.index(c["answer"])
                t = time.perf_counter(); sc.llm.reset(); sc.llm.eval(pre + suf); lg = sc.logits(ids); cold.append(time.perf_counter() - t)
                ok["argmax"] += int(np.argmax(lg)) == gold
                if r == 0 and int(np.argmax(lg)) != gold:
                    wrong.append(f"{c['id']}:{order[int(np.argmax(lg))]}")
                if a.generate:
                    sc.llm.reset()
                    t = time.perf_counter(); sc.llm.eval(pre + suf); g = sc.generate(tag); gen.append(time.perf_counter() - t)
                    ok["generate"] += g == gold
                sc.llm.reset(); sc.llm.eval(pre)  # the fixed part, paid once per question, not per decision
                if a.calib:
                    sc.llm.eval(null); base = sc.logits(ids); sc.rewind(len(pre))
                t = time.perf_counter(); sc.rewind(len(pre)); sc.llm.eval(suf); lg2 = sc.logits(ids); cached.append(time.perf_counter() - t)
                if a.calib:
                    ok["calib"] += int(np.argmax(lg2 - base)) == gold
            runs.append(ok)
    n = len(cases)
    res = {"label": a.label, "model": os.path.basename(a.model), "cases": os.path.basename(a.cases), "n_cases": n,
           "threads": a.threads, "reps": a.reps, "shuffled": not a.no_shuffle, "label_by": a.label_by,
           "shots": len(shots), "volatile": a.volatile, "wrong": wrong,
           "correct_argmax": runs[0]["argmax"], "correct_calib": runs[0]["calib"] if a.calib else None,
           "correct_generate": runs[0]["generate"] if a.generate else None,
           "deterministic": all(r == runs[0] for r in runs),
           "prompt_tokens_mean": round(statistics.mean(len(b[0]) + len(b[1]) for b in built), 1),
           "suffix_tokens_mean": round(statistics.mean(len(b[1]) for b in built), 1),
           "cold_ms": stats(cold), "cached_ms": stats(cached), "generate_ms": stats(gen) if gen else None,
           "idle": idle, "during": mon.summary()}
    print(json.dumps(res))
    if a.out:
        os.makedirs(os.path.dirname(os.path.abspath(a.out)), exist_ok=True)
        json.dump(res, open(a.out, "w"), indent=1)


if __name__ == "__main__":
    main()
