"""Pairs, splits and the one-line text a model reads.

Real pairs come from extract.py (untracked, bench/decider/data). Split by time, whole sessions:
  train      sessions of 2026-11-14
  test_real  sessions of 2026-11-15 (never used for fitting, model choice or thresholds)
  test_hand  cases_nochat.jsonl: hand-written, no chat line, written before any training
"""
import json, os, re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
DATA = os.path.join(ROOT, "data")
TEST_DAY = "20261115"


def load(path):
    return [json.loads(l) for l in open(path)]


def real():
    ps = load(os.path.join(DATA, "pairs_real.jsonl"))
    return [p for p in ps if not p["session"].startswith(TEST_DAY)], [p for p in ps if p["session"].startswith(TEST_DAY)]


def hand():
    return load(os.path.join(ROOT, "cases_nochat.jsonl"))


def synth(kind):
    path = os.path.join(DATA, f"pairs_synth_{kind}.jsonl")
    return load(path) if os.path.exists(path) else []


def event_text(e):
    """Event without tick stamps and goal ids: they carry no decision and cost tokens."""
    e = re.sub(r"\bt=\d+ ", "", e)
    e = re.sub(r"#\d+ ", "", e)
    return e.split(". Take the next step")[0].split(". The body has no goal")[0]


def snap_text(s):
    """Snapshot without tick, position and digest."""
    s = re.sub(r"^t=\d+ pos \S+ ", "", s)
    s = re.sub(r" ; digest:.*$", "", s)
    s = re.sub(r" ; \d+ events pending.*$", "", s)
    return re.sub(r"#\d+ ", "", s)


def text(p):
    did = " > ".join(p["history"][-3:]) or "nothing yet"
    return f"event: {event_text(p['event'])} | did: {did} | body: {snap_text(p['snapshot'])}"


def detail_label(p):
    """Goal plus item, without counts or coordinates: 'craft stick', 'place oak_planks', 'go_to'."""
    d = p.get("detail", p["label"]).split()
    return " ".join(w for w in d if not w.isdigit() and w != "pos")
