"""Backend adapters: each exposes load() and predict(state, questions) -> {qid: wire answer dict}.

Wire answer shapes (TypeSafe /v1/systemone): choice -> {"choice": key}, noul -> {"noul": p_yes},
score -> {"score": expected level}.  Von, Laya and Kev speak this natively; Jeff is adapted below.
"""
import json
import os
import urllib.request

THIRD = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "third_party", "decider")


class VonBackend:
    name = "von"

    def load(self, **kw):
        from von.engine import VonEngine
        self.eng = VonEngine(device="cpu")

    def predict(self, state, questions):
        resp = self.eng.evaluate(state=state, questions=questions)
        out = {}
        for k, a in resp.answers.items():
            out[k] = {"choice": getattr(a, "choice", None), "noul": getattr(a, "noul", None),
                      "score": getattr(a, "score", None)}
        return out


class LayaBackend:
    name = "laya"

    def load(self, model="convaiinnovations/laya", vega=None, **kw):
        import laya
        self.agent = laya.Agent(vegaml=vega) if vega else laya.Agent(model)

    def predict(self, state, questions):
        return self.agent.predict(state, questions)["answers"]


class HttpBackend:
    """Any server exposing POST /v1/systemone (Kev, or von/laya serve)."""
    name = "http"

    def load(self, url="http://127.0.0.1:8009", model="kev-latest", **kw):
        self.url, self.model = url.rstrip("/"), model

    def predict(self, state, questions):
        body = json.dumps({"state": state, "model": self.model, "questions": questions}).encode()
        req = urllib.request.Request(self.url + "/v1/systemone", body, {"content-type": "application/json"})
        with urllib.request.urlopen(req, timeout=300) as r:
            return json.load(r)["answers"]


class JeffBackend:
    """No general Jeff decider exists for this task: only task-specific adapters. The closest is the open router
    adapter (jeff.routing.router-open-v14), which takes instructions + caller options + state and picks one option.
    yes/no and score questions are posed to it as 2- and N-way choices."""
    name = "jeff"
    FN = "jeff.routing.router-open-v14.specialist"

    def load(self, **kw):
        import jeff
        self.client = jeff.Jeff.local(["router-open-v14"])
        self.fns = {}

    def _fn(self, labels):
        key = tuple(labels)
        if key not in self.fns:
            self.fns[key] = self.client.function(self.FN, labels={k: labels[k] for k in labels})
        return self.fns[key]

    def predict(self, state, questions):
        out = {}
        for qid, q in questions.items():
            t = q["type"]
            if t == "choice":
                labels = dict(q["criteria"])
            elif t == "noul":
                labels = {"no": "No", "yes": "Yes"}
            else:
                labels = {str(i): s for i, s in enumerate(q["criteria"])}
            req = {"instructions": q["instructions"], "options": list(labels), "state": state}
            d = self._fn(labels)(req)
            probs = getattr(d, "probabilities", None) or {}
            if t == "choice":
                out[qid] = {"choice": d.value}
            elif t == "noul":
                out[qid] = {"noul": float(probs.get("yes", 1.0 if d.value == "yes" else 0.0))}
            else:
                out[qid] = {"score": sum(int(k) * float(v) for k, v in probs.items()) if probs else int(d.value)}
        return out


BACKENDS = {"von": VonBackend, "laya": LayaBackend, "http": HttpBackend, "jeff": JeffBackend}
