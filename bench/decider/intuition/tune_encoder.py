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


def folds(pairs, k):
    """Whole sessions to k folds, round-robin in time order. -> list of (train, test)."""
    names = sorted({p["session"] for p in pairs})
    part = {n: i % k for i, n in enumerate(names)}
    return [([p for p in pairs if part[p["session"]] != f], [p for p in pairs if part[p["session"]] == f]) for f in range(k)]


def fit(train, a, rng, log=False):
    """Train a fresh encoder and head on `train`. -> (predict(pairs) -> labels, confidences; parts to save)."""
    from transformers import AutoModel, AutoTokenizer
    tok, enc = AutoTokenizer.from_pretrained(MODEL), AutoModel.from_pretrained(MODEL)
    labels = sorted({p["label"] for p in train})
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
        if log:
            print(f"epoch {ep + 1} loss {tot / len(train):.3f} {time.time() - t0:.0f}s", flush=True)
    enc.eval()

    @torch.inference_mode()
    def predict(ps):
        P = torch.cat([torch.softmax(logits(ps[i:i + 32]), -1) for i in range(0, len(ps), 32)])
        return [labels[i] for i in P.argmax(1)], [float(c) for c in P.max(1).values]
    return predict, (tok, enc, head, labels)


def cross(a):
    """--folds K: K-fold over all real sessions, so the trained encoder has a view beside the one test day."""
    torch.set_num_threads(a.threads); torch.manual_seed(0); rng = random.Random(0)
    tr, te = data.real()
    pred, conf, gold, sess = [], [], [], []
    for i, (train, test) in enumerate(folds(tr + te, a.folds)):
        pr, cf = fit(train, a, rng)[0](test)
        pred += pr; conf += cf; gold += [p["label"] for p in test]; sess += [(p["session"], p["i"]) for p in test]
        print("fold", i, len(train), len(test), sum(x == y for x, y in zip(pr, [p["label"] for p in test])), flush=True)
    res = {"arm": "a4 encoder trained end to end", "folds": a.folds, "epochs": a.epochs, "cases": sess,
           "all_real": score(pred, conf, gold, None)}
    json.dump(res, open(a.out, "w"), indent=1)


def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--folds", type=int, default=0)
    ap.add_argument("--set", choices=("real", "real+rules", "real+vary+rules"), default="real")
    ap.add_argument("--epochs", type=int, default=10); ap.add_argument("--lr", type=float, default=5e-5)
    ap.add_argument("--threads", type=int, default=4); ap.add_argument("--out", required=True)
    ap.add_argument("--save", default=None, help="directory for the trained weights (outside the repository)")
    a = ap.parse_args()
    if a.folds:
        return cross(a)
    torch.set_num_threads(a.threads); torch.manual_seed(0); rng = random.Random(0)
    train, test_real = data.real()
    hand = data.hand()
    train = train + (data.synth("vary") if "vary" in a.set else []) + (data.synth("rules") if "rules" in a.set else [])
    t0 = time.time()
    predict, (tok, enc, head, labels) = fit(train, a, rng, log=True)
    if a.save:                                  # before scoring: a failed evaluation must not cost the training
        os.makedirs(a.save, exist_ok=True)
        enc.save_pretrained(a.save); tok.save_pretrained(a.save)
        torch.save({"head": head.state_dict(), "labels": labels}, os.path.join(a.save, "head.pt"))
    res = {"arm": "a4 encoder trained end to end", "set": a.set, "epochs": a.epochs, "n_train": len(train),
           "train_s": round(time.time() - t0), "labels": labels}
    for name, ps in (("test_real", test_real), ("test_hand", hand)):
        pr, cf = predict(ps)
        res[name] = score(pr, cf, [p["label"] for p in ps], None)
        print(name, res[name]["k"], "/", res[name]["n"], res[name]["cov95_oracle"])
    json.dump(res, open(a.out, "w"), indent=1)


if __name__ == "__main__":
    main()
