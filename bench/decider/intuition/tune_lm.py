"""Arm (d): the 135M language model with a low-rank adapter, trained to put the decision after 'Answer:'.

  python tune_lm.py --set real --epochs 8 --out ../results/trained/lm135-lora-real.json

The prompt is the pair text and nothing else: no option list and no solved examples, the adapter carries them.
The loss and the decision are both taken over the first token of each label at the answer position (argmax).
"""
import argparse, json, os, random, sys, time
import torch
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import data
from classic import score

BASE = os.path.expanduser("~/models/intuition/hf/SmolLM2-135M-Instruct")
ALIAS = {"set_mode": "mode", "sleep_in_bed": "bed", "eat_now": "eat", "go_to": "go", "come_here": "come"}


def prompt(p):
    return f"{data.text(p)}\nAnswer:"


def label_ids(tok, labels):
    ids = [tok.encode(" " + ALIAS.get(l, l), add_special_tokens=False)[0] for l in labels]
    assert len(set(ids)) == len(ids), "two labels share a first token"
    return ids


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--set", choices=("real", "real+vary", "real+vary+rules"), default="real")
    ap.add_argument("--epochs", type=int, default=8); ap.add_argument("--lr", type=float, default=5e-4)
    ap.add_argument("--rank", type=int, default=16); ap.add_argument("--threads", type=int, default=4)
    ap.add_argument("--out", required=True); ap.add_argument("--save", default=None)
    a = ap.parse_args()
    from peft import LoraConfig, get_peft_model
    from transformers import AutoModelForCausalLM, AutoTokenizer
    torch.set_num_threads(a.threads); torch.manual_seed(0); rng = random.Random(0)
    train, test_real = data.real()
    hand = data.hand()
    train = train + (data.synth("vary") if "vary" in a.set else []) + (data.synth("rules") if "rules" in a.set else [])
    labels = sorted({p["label"] for p in train})
    tok = AutoTokenizer.from_pretrained(BASE); tok.padding_side = "left"
    tok.pad_token = tok.pad_token or tok.eos_token
    ids = label_ids(tok, labels)
    model = get_peft_model(AutoModelForCausalLM.from_pretrained(BASE, torch_dtype=torch.float32), LoraConfig(
        r=a.rank, lora_alpha=2 * a.rank, lora_dropout=0.05, task_type="CAUSAL_LM",
        target_modules=["q_proj", "k_proj", "v_proj", "o_proj", "gate_proj", "up_proj", "down_proj"]))
    opt = torch.optim.AdamW([p for p in model.parameters() if p.requires_grad], lr=a.lr)

    def logits(ps):
        x = tok([prompt(p) for p in ps], padding=True, truncation=True, max_length=320, return_tensors="pt")
        return model(**x).logits[:, -1, ids]

    order = sorted(range(len(train)), key=lambda i: len(prompt(train[i])))   # similar lengths per batch
    chunks = [order[i:i + 16] for i in range(0, len(order), 16)]
    steps, total, t0 = 0, a.epochs * len(chunks), time.time()
    for ep in range(a.epochs):
        model.train(); rng.shuffle(chunks); tot = 0.0
        for b in chunks:
            for g in opt.param_groups:
                g["lr"] = a.lr * (1 - steps / total)
            loss = torch.nn.functional.cross_entropy(logits([train[i] for i in b]),
                                                     torch.tensor([labels.index(train[i]["label"]) for i in b]))
            opt.zero_grad(); loss.backward(); opt.step(); tot += loss.item() * len(b); steps += 1
        print(f"epoch {ep + 1} loss {tot / len(train):.3f} {time.time() - t0:.0f}s", flush=True)
    model.eval()

    @torch.inference_mode()
    def predict(ps):
        P = torch.cat([torch.softmax(logits(ps[i:i + 16]), -1) for i in range(0, len(ps), 16)])
        return [labels[i] for i in P.argmax(1)], [float(c) for c in P.max(1)]

    res = {"arm": "d 135M + low-rank adapter", "set": a.set, "epochs": a.epochs, "rank": a.rank, "n_train": len(train),
           "train_s": round(time.time() - t0), "labels": labels,
           "prompt_tokens_mean": round(sum(len(tok.encode(prompt(p))) for p in test_real) / len(test_real), 1)}
    for name, ps in (("test_real", test_real), ("test_hand", hand)):
        pr, cf = predict(ps)
        res[name] = score(pr, cf, [p["label"] for p in ps], None)
        print(name, res[name]["k"], "/", res[name]["n"], res[name]["cov95_oracle"])
    lat = []
    with torch.inference_mode():   # one decision at a time, as deployed; this framework, not the fast runtime
        for p in (test_real + hand)[:60]:
            t = time.perf_counter(); logits([p]); lat.append(time.perf_counter() - t)
    lat.sort(); res["torch_ms"] = {"p50": round(lat[len(lat) // 2] * 1e3), "p90": round(lat[int(len(lat) * .9)] * 1e3),
                                   "note": "ungated, unmerged adapter"}
    json.dump(res, open(a.out, "w"), indent=1)
    if a.save:
        model.save_pretrained(a.save)


if __name__ == "__main__":
    main()
