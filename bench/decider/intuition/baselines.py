"""Floors: uniform chance, majority label of the training set, repeating the last decision, and yesterday's
keyword rule (trivial.py) fed the real state in its old layout."""
import collections, os, re, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from trivial import rule


def majority(train):
    return collections.Counter(p["label"] for p in train).most_common(1)[0][0]


def repeat_last(p):
    """Do again what was done last (a goal name; 'wait' when there is no history)."""
    return p["history"][-1].split()[0] if p["history"] else "wait"


def old_layout(p):
    sn = p["snapshot"]
    g = lambda pat, d: (re.search(pat, sn) or [None, d])[1]
    chat = re.search(r"chat: <[^>]*> (.*)", p["event"])
    return (f"HP {g(r'hp (\d+)', 20)}/20 | Hunger {g(r'food (\d+)', 20)}/20 | Held: {g(r'held (\w+)', 'nothing')} | "
            f"Nearby: {g(r'near: ([^;]*)', 'none')} | Time: {'night' if ' night ' in sn else 'day'} | "
            f"Companion: {g(r'companion ([^;]*)', 'not visible')} | Chat: {chat.group(1) if chat else 'none'} | "
            f"Goal: {g(r'; goal ([^;]*)', 'none')} | Event: {p['event']}")


def keyword(p):
    return rule({"state": old_layout(p)})
