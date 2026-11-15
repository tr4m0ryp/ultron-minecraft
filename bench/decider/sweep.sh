#!/usr/bin/env bash
# Variant sweep for the Intuition layer on quantized small models (logit_run.py) and a sentence encoder
# (embed_run.py). Weights live outside the repository, in $GGUF.
#   bench/decider/sweep.sh quant threads small layout goals full embed
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
GGUF="${GGUF:-$HOME/models/intuition/gguf}"
PY="${PY:-$HOME/models/intuition/venv/bin/python}"
EPY="${EPY:-$HERE/../../third_party/decider/kev/.venv/bin/python}"   # any environment with torch and transformers
OUT="$HERE/results/sweep"; mkdir -p "$OUT"
S135=SmolLM2-135M-Instruct; S360=SmolLM2-360M-Instruct; Q05=qwen2.5-0.5b-instruct
# 16-bit float weights are the fastest format on this CPU (AVX without AVX2), see the quant phase
BEST135="${BEST135:-$S135-f16}"; BEST360="${BEST360:-$S360-f16}"; BEST05="${BEST05:-$Q05-fp16}"

run() {  # label file-stem extra args...
  local label=$1 stem=$2; shift 2
  [ -s "$OUT/$label.json" ] && return
  "$PY" "$HERE/logit_run.py" --model "$GGUF/$stem.gguf" --label "$label" --gate-wait "${GATE_WAIT:-45}" --out "$OUT/$label.json" "$@" >/dev/null 2>"$OUT/$label.err" || echo "failed: $label"
}
M20="--stride 3 --reps 3"

for phase in "$@"; do case $phase in
quant)    # (a) 4-bit, 5-bit, 8-bit and 16-bit float weights; (b) argmax against generated text; (c) cached prefix
  for q in Q4_K_M Q5_K_M Q8_0 f16; do
    x=""; [ $q = Q4_K_M ] && x="--generate --calib"
    run "smol135-$q" "$S135-$q" $M20 $x; run "smol360-$q" "$S360-$q" $M20 $x
  done
  for q in q4_k_m q5_k_m q8_0 fp16; do
    x=""; [ $q = q4_k_m ] && x="--generate --calib"
    run "qwen05-$q" "$Q05-$q" $M20 $x
  done ;;
threads)  # (d)
  for t in 2 8; do run "smol135-best-t$t" "$BEST135" $M20 --threads $t; run "qwen05-best-t$t" "$BEST05" $M20 --threads $t; done ;;
small)    # (e) other small models, and one larger model as an accuracy reference
  run gemma270-f16 gemma-3-270m-it-F16 $M20; run lfm350-f16 LFM2-350M-F16 $M20
  run qwen3-06-Q8_0 Qwen3-0.6B-Q8_0 $M20; run qwen15-Q4_K_M Qwen2.5-1.5B-Instruct-Q4_K_M --stride 3 --reps 1 ;;
layout)   # solved examples in the cached prefix; only the volatile state fields evaluated per decision
  for m in "smol135 $BEST135" "smol360 $BEST360" "qwen05 $BEST05"; do set -- $m
    run "$1-shots" "$2" $M20 --shots "$HERE/shots/mixed.jsonl"
    run "$1-shots-volatile" "$2" $M20 --shots "$HERE/shots/mixed.jsonl" --volatile
  done ;;
goals)    # 65 cases over the 13 real goals, one fixed question
  G="--cases $HERE/cases_goals.jsonl --no-shuffle"   # one fixed option order, as it would be deployed
  for m in "smol135 $BEST135" "smol360 $BEST360" "qwen05 $BEST05" "qwen15 Qwen2.5-1.5B-Instruct-Q4_K_M"; do set -- $m
    [ $1 = qwen15 ] || run "goals-$1" "$2" $G --reps 1 --calib   # the 1.5B plain run takes 20 minutes and decides nothing
    run "goals-$1-shots-key" "$2" $G --reps 1 --shots "$HERE/shots/goals.jsonl" --label-by key --calib
    run "goals-$1-shots-key-volatile" "$2" $G --reps 3 --shots "$HERE/shots/goals.jsonl" --label-by key --volatile --calib
  done ;;
full)     # all 60 mixed cases, accuracy
  for m in "smol135 $BEST135" "smol360 $BEST360" "qwen05 $BEST05" "qwen15 Qwen2.5-1.5B-Instruct-Q4_K_M"; do set -- $m
    run "full60-$1" "$2" --reps 1 --calib
  done ;;
embed)
  e() { [ -s "$OUT/$1.json" ] || "$EPY" "$HERE/embed_run.py" --label "$1" --out "$OUT/$1.json" --gate-wait "${GATE_WAIT:-45}" "${@:2}" >/dev/null 2>"$OUT/$1.err" || echo "failed: $1"; }
  e embed-mixed20 --stride 3; e embed-mixed60 --reps 1
  e embed-goals --cases "$HERE/cases_goals.jsonl" --shots "$HERE/shots/goals.jsonl"
  for t in 2 8; do e "embed-goals-t$t" --cases "$HERE/cases_goals.jsonl" --shots "$HERE/shots/goals.jsonl" --threads $t; done ;;
esac; done
echo finished
