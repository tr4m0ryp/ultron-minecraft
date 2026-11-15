#!/usr/bin/env bash
# Baseline rerun of the open decision models, same settings as the first run (every 3rd case, 20 calls,
# 3 five-question batches), repeated, each run behind the idle gate. Then a thread sweep.
#   bench/decider/rerun.sh [reps]
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
TP="$HERE/../../third_party/decider"
OUT="$HERE/results/rerun"
REPS="${1:-3}"
export HF_HOME="$TP/.hf" HF_HUB_OFFLINE=1 TOKENIZERS_PARALLELISM=false
mkdir -p "$OUT"
COMMON="--stride 3 --n-lat 20 --n-batch 3 --gate-wait ${GATE_WAIT:-240}"

laya() {  # label threads
  "$TP/laya/.venv/bin/python" "$HERE/run.py" --backend laya --label "$1" --threads "$2" $COMMON --out "$OUT/$1.json" >/dev/null 2>"$OUT/$1.log"
}
kev() {  # label threads ("" = server default)
  ( cd "$TP/kev" && exec env ${2:+OMP_NUM_THREADS=$2} .venv/bin/python -m kev.serve --run jaredpalmer/kev-0.8b --port 8009 ) >"$OUT/$1.serve.log" 2>&1 &
  local pid=$!
  until curl -sf -m 2 http://127.0.0.1:8009/v1/models >/dev/null; do sleep 2; kill -0 $pid 2>/dev/null || return 1; done
  "$TP/kev/.venv/bin/python" "$HERE/run.py" --backend http --url http://127.0.0.1:8009 --model kev-latest --label "$1" --threads "${2:-4}" --pid $pid $COMMON --out "$OUT/$1.json" >/dev/null 2>"$OUT/$1.log"
  kill $pid; wait $pid 2>/dev/null
}
jeff() {  # label; the first run used every 15th case (4 cases), it is too slow for more
  "$TP/jeff/.venv/bin/python" "$HERE/run.py" --backend jeff --label "$1" --threads 8 --stride 15 --n-lat 4 --n-batch 3 --gate-wait "${GATE_WAIT:-240}" --out "$OUT/$1.json" >/dev/null 2>"$OUT/$1.log"
}

for i in $(seq 1 "$REPS"); do laya "laya-421M-t8-r$i" 8; done
for i in $(seq 1 "$REPS"); do kev "kev-0.8b-t4-r$i" ""; done
for t in 2 4; do laya "laya-421M-t$t-r1" $t; done
for t in 2 8; do kev "kev-0.8b-t$t-r1" $t; done
for i in $(seq 1 "$REPS"); do jeff "jeff-router-r$i"; done
echo finished
