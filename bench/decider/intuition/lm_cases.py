"""Held-out pairs in the case layout of logit_run.py, so yesterday's untrained variants (135M and 1.5B with
solved examples in the cached prefix, argmax over option names) run unchanged on the new test sets.

  python lm_cases.py     -> data/lm_test_real.jsonl, data/lm_test_hand.jsonl, data/lm_shots.jsonl

Solved examples: the first training pair of every label seen in training (12). Option names are shortened
where two would share a first token.
"""
import json, os
from data import DATA, hand, real, text

OPTIONS = {
    "place": "Put a block, torch or workstation down", "craft": "Make an item from materials in the inventory",
    "go": "Travel to a place or coordinates", "wait": "Do nothing, the running goal carries on",
    "mine": "Dig out a block, ore or tree", "collect": "Pick up items lying on the ground",
    "smelt": "Smelt ore in a furnace", "attack": "Fight or hunt a specific mob", "say": "Answer in chat",
    "mode": "Change temperament, for example retreat", "inhibit": "Switch one reflex off here",
    "give": "Hand an item to the companion", "follow": "Keep walking with the companion",
    "end": "Cancel the running goal", "guard": "Protect the companion", "bed": "Sleep in a nearby bed",
    "eat": "Eat food right away", "arm": "Prepare an automatic response to a coming danger"}
SHORT = {"go_to": "go", "set_mode": "mode", "sleep_in_bed": "bed", "eat_now": "eat"}


def case(p, i):
    return {"kind": "goal", "id": p.get("id", f"r{i:03d}"), "state": text(p), "answer": SHORT.get(p["label"], p["label"]),
            "question": {"type": "choice", "instructions": "What should the player do now?", "criteria": OPTIONS}}


def main():
    train, test = real()
    seen, shots = set(), []
    for p in train:
        if p["label"] not in seen:
            seen.add(p["label"]); shots.append(p)
    for name, ps in (("lm_test_real", test), ("lm_test_hand", hand()), ("lm_shots", shots)):
        with open(os.path.join(DATA, name + ".jsonl"), "w") as f:
            f.writelines(json.dumps(case(p, i)) + "\n" for i, p in enumerate(ps))
        print(name, len(ps))


if __name__ == "__main__":
    main()
