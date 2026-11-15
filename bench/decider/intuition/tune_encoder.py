"""Arm (a4): the 33M sentence encoder trained end to end on the pairs, a linear head on its CLS vector.

  python tune_encoder.py --set real --epochs 10 --out ../results/trained/encoder-ft-real.json
"""
import argparse, json, os, random, sys, time
import torch
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import data
from classic import score
from embed import MODEL


def batches(n, size, rng):
    idx = list(range(n)); rng.shuffle(idx)
    return [idx[i:i + size] for i in range(0, n, size)]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--set", choices=("real", "real+vary", "real+vary+rules"), default="real")
    ap.add_argument("--epochs", type=int, default=10); ap.add_argument("--lr", type=float, default=5e-5)
    ap.add_argument("--threads", type=int, default=4); ap.add_argument("--out", required=True)
    ap.add_argument("--save", default=None, help="directory for the trained weights (outside the repository)")
    a = ap.parse_args()
    from transformers import AutoModel, AutoTokenizer
    torch.set_num_threads(a.threads); torch.manual_seed(0); rng = random.Random(0)
    train, test_real = data.real()
    hand = data.hand()
    train = train + (data.synth("vary") if "vary" in a.set else []) + (data.synth("rules") if "rules" in a.set else [])
    labels = sorted({p["label"] for p in train})
    tok, enc = AutoTokenizer.from_pretrained(MODEL), AutoModel.from_pretrained(MODEL)
    head = torch.nn.Linear(enc.config.hidden_size, len(labels))
    opt = torch.optim.AdamW([{"params": enc.parameters(), "lr": a.lr}, {"params": head.parameters(), "lr": 1e-3}])

    def logits(ps):
        x = tok([data.text(p) for p in ps], padding=True, truncation=True, max_length=256, return_tensors="pt")
        return head(enc(**x).last_hidden_state[:, 0])

    t0 = time.time()
    for ep in range(a.epochs):
        enc.train(); tot = 0.0
        for b in batches(len(train), 16, rng):
            loss = torch.nn.functional.cross_entropy(logits([train[i] for i in b]),
                                                     torch.tensor([labels.index(train[i]["label"]) for i in b]))
            opt.zero_grad(); loss.backward(); opt.step(); tot += loss.item() * len(b)
        print(f"epoch {ep + 1} loss {tot / len(train):.3f} {time.time() - t0:.0f}s", flush=True)
    enc.eval()

    @torch.inference_mode()
    def predict(ps):
        P = torch.cat([torch.softmax(logits(ps[i:i + 32]), -1) for i in range(0, len(ps), 32)])
        return [labels[i] for i in P.argmax(1)], [float(c) for c in P.max(1)]

    res = {"arm": "a4 encoder trained end to end", "set": a.set, "epochs": a.epochs, "n_train": len(train),
           "train_s": round(time.time() - t0), "labels": labels}
    for name, ps in (("test_real", test_real), ("test_hand", hand)):
        pr, cf = predict(ps)
        res[name] = score(pr, cf, [p["label"] for p in ps], None)
        print(name, res[name]["k"], "/", res[name]["n"], res[name]["cov95_oracle"])
    json.dump(res, open(a.out, "w"), indent=1)
    if a.save:
        os.makedirs(a.save, exist_ok=True)
        enc.save_pretrained(a.save); tok.save_pretrained(a.save)
        torch.save({"head": head.state_dict(), "labels": labels}, os.path.join(a.save, "head.pt"))


if __name__ == "__main__":
    main()
