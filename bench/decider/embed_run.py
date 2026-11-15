"""Smallest size class: a 33M sentence encoder instead of a generative model.

  python embed_run.py --cases cases_goals.jsonl --shots shots/goals.jsonl --out results/embed-goals.json

One forward pass over the state text per decision; everything else is embedded once. Three ways to decide:
  zero   cosine between "instruction + state" and each option description (works for any case)
  shots  nearest solved example from --shots (goal cases: 13 examples, one per goal, never in the test set)
  loo    nearest other test case, leave-one-out (goal cases; optimistic, the neighbours share an author)
"""
import argparse, json, os, statistics, sys, time
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import torch
from idle import Monitor, gate
from logit_run import stats
from trivial import options


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", default="BAAI/bge-small-en-v1.5"); ap.add_argument("--label", default="bge-small-33M")
    ap.add_argument("--cases", default=os.path.join(HERE, "cases.jsonl")); ap.add_argument("--stride", type=int, default=1)
    ap.add_argument("--shots", default=None); ap.add_argument("--threads", type=int, default=4)
    ap.add_argument("--reps", type=int, default=3); ap.add_argument("--out", default=None)
    ap.add_argument("--gate-wait", type=int, default=60)
    a = ap.parse_args()
    from transformers import AutoModel, AutoTokenizer
    torch.set_num_threads(a.threads)
    tok, model = AutoTokenizer.from_pretrained(a.model), AutoModel.from_pretrained(a.model).eval()

    @torch.inference_mode()
    def emb(texts):
        x = tok(texts, padding=True, truncation=True, max_length=256, return_tensors="pt")
        return torch.nn.functional.normalize(model(**x).last_hidden_state[:, 0], dim=-1)

    cases = [json.loads(l) for l in open(a.cases)][::a.stride]
    shots = [json.loads(l) for l in open(a.shots)] if a.shots else []

    def desc(c):
        q = c["question"]
        if q["type"] == "noul":
            return ["Yes, " + q["instructions"], "No, not: " + q["instructions"]]
        return [f"{k}: {v}" for k, v in q["criteria"].items()] if q["type"] == "choice" else list(q["criteria"])

    opt = [emb(desc(c)) for c in cases]            # fixed per question: embedded once
    shot_e = emb([s["state"] for s in shots]) if shots else None
    emb([cases[0]["state"]])                       # warm-up
    idle = gate(wait_s=a.gate_wait)
    lat, zero, near, vecs = [], 0, 0, []
    with Monitor() as mon:
        for r in range(a.reps):
            for c, oe in zip(cases, opt):
                t = time.perf_counter()
                q = emb([c["question"]["instructions"] + " " + c["state"]])
                pick = int((q @ oe.T).argmax())
                lat.append(time.perf_counter() - t)
                if r == 0:
                    zero += options(c)[pick] == c["answer"]
                    s = emb([c["state"]]); vecs.append(s)
                    if shots:
                        near += shots[int((s @ shot_e.T).argmax())]["answer"] == c["answer"]
    res = {"label": a.label, "model": a.model, "cases": os.path.basename(a.cases), "n_cases": len(cases),
           "threads": a.threads, "reps": a.reps, "correct_zero": zero, "correct_shots": near if shots else None,
           "n_shots": len(shots), "ms": stats(lat), "idle": idle, "during": mon.summary()}
    if all(c["kind"] == "goal" for c in cases):
        v = torch.cat(vecs); sim = v @ v.T; sim.fill_diagonal_(-1)
        res["correct_loo"] = sum(cases[int(j)]["answer"] == c["answer"] for c, j in zip(cases, sim.argmax(1)))
    print(json.dumps(res))
    if a.out:
        json.dump(res, open(a.out, "w"), indent=1)


if __name__ == "__main__":
    main()
