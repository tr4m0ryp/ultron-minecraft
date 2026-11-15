"""Trained Intuition: does a small model trained on recorded Thought decisions fill the layer? (measurement only)

Scripts, run by path through ../trained.sh:
  extract.py      session transcripts -> (state, decision) pairs        synth.py     labelled synthetic pairs
  floors.py       chance, majority, repeat-last, keyword rule           classic.py   arms a1-a3, b1, b5, c1, c2
  habit.py        arm c3, a count table                                 tune_encoder.py  arm a4
  tune_lm.py      arm d, low-rank adapter on the 135M model             wake_filter.py   wait against act
  folds.py        cheap arms on 3 session folds                         latency.py   per-decision timing
  lm_cases.py     test pairs in the layout of ../logit_run.py           report.py    result tables
"""
from .data import hand, real, synth, text
from .features import features
from .metrics import acc, coverage_at, curve, wilson
