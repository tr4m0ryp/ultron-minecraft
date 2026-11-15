"""Sentence-encoder embeddings of pair texts (CLS token, L2-normalised), cached on disk by text hash."""
import hashlib, os
import numpy as np
import torch
from data import DATA, text

MODEL = "BAAI/bge-small-en-v1.5"


class Encoder:
    def __init__(self, model=MODEL, threads=4, cache=True):
        from transformers import AutoModel, AutoTokenizer
        torch.set_num_threads(threads)
        self.tok, self.model = AutoTokenizer.from_pretrained(model), AutoModel.from_pretrained(model).eval()
        self.path = os.path.join(DATA, "emb-" + model.split("/")[-1] + ".npz") if cache else None
        self.cache = dict(np.load(self.path)) if self.path and os.path.exists(self.path) else {}

    @torch.inference_mode()
    def one(self, s):
        x = self.tok([s], truncation=True, max_length=256, return_tensors="pt")
        return torch.nn.functional.normalize(self.model(**x).last_hidden_state[:, 0], dim=-1)[0].numpy()

    def tokens(self, s):
        return len(self.tok(s, truncation=True, max_length=256)["input_ids"])

    def many(self, pairs):
        out, new = [], 0
        for p in pairs:
            s = text(p)
            k = hashlib.sha1(s.encode()).hexdigest()
            if k not in self.cache:
                self.cache[k] = self.one(s); new += 1
            out.append(self.cache[k])
        if new and self.path:
            np.savez(self.path, **self.cache)
        return np.stack(out)
