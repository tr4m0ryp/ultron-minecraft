#!/usr/bin/env bash
# Trained-Intuition measurement: pairs from recorded Thought decisions, then every arm, in sequence.
#   bench/decider/trained.sh data floors classic plan habit probe encoder lora untrained latency
# Weights live outside the repository ($GGUF, $HF); pairs and logs are ignored by git (data/, logs/).
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"; I="$HERE/intuition"
GGUF="${GGUF:-$HOME/models/intuition/gguf}"
PY="${PY:-$HOME/models/intuition/venv/bin/python}"                     # llama.cpp binding
EPY="${EPY:-$HERE/../../third_party/decider/kev/.venv/bin/python}"     # torch, transformers, scikit-learn, peft
OUT="$HERE/results/trained"; LOG="$HERE/logs"; mkdir -p "$OUT" "$LOG"
export OMP_NUM_THREADS=4 OPENBLAS_NUM_THREADS=4 MKL_NUM_THREADS=4 TOKENIZERS_PARALLELISM=false
S135=SmolLM2-135M-Instruct-f16; Q15=Qwen2.5-1.5B-Instruct-Q4_K_M

step() { local name=$1; shift; [ -s "$OUT/$name.json" ] && return; "$@" --out "$OUT/$name.json" >"$LOG/$name.log" 2>&1 || echo "failed: $name"; }
old() {  # label model cases: yesterday's untrained variant (examples in the cached prefix, argmax over option names)
  step "$1" "$PY" "$HERE/logit_run.py" --model "$GGUF/$2.gguf" --label "$1" --cases "$HERE/data/$3.jsonl" --no-shuffle \
    --reps 1 --shots "$HERE/data/lm_shots.jsonl" --n-shots 12 --label-by key --calib --gate-wait "${GATE_WAIT:-120}"
}

for phase in "$@"; do case $phase in
data)      (cd "$HERE/../.." && python3 "$I/extract.py") && (cd "$I" && python3 synth.py && python3 lm_cases.py) ;;
classic)   step classic "$EPY" -u "$I/classic.py"; step classic-detail "$EPY" -u "$I/classic.py" --label detail --no-loso ;;
plan)      INTUITION_PLAN=1 step classic-plan "$EPY" -u "$I/classic.py" ;;
probe)     step classic-lm135-probe "$EPY" -u "$I/classic.py" --encoder lm135 --arms a1,a2,b5 --sets real ;;
floors)    step floors "$EPY" -u "$I/floors.py"; step wake-filter "$EPY" -u "$I/wake_filter.py" ;;
habit)     step habit "$EPY" -u "$I/habit.py" ;;
encoder)   step encoder-ft-real "$EPY" -u "$I/tune_encoder.py" --set real --epochs 10
           step encoder-ft-rules "$EPY" -u "$I/tune_encoder.py" --set real+rules --epochs 3 ;;
lora)      step lm135-lora-real "$EPY" -u "$I/tune_lm.py" --set real --epochs 5
           step lm135-lora-rules "$EPY" -u "$I/tune_lm.py" --set real+rules --epochs 3 ;;
untrained) old untrained-smol135-real $S135 lm_test_real; old untrained-smol135-hand $S135 lm_test_hand
           old untrained-qwen15-real $Q15 lm_test_real; old untrained-qwen15-hand $Q15 lm_test_hand ;;
latency)   step latency-classic "$EPY" -u "$I/latency.py" classic --gate-wait "${GATE_WAIT:-300}"
           step latency-lm "$PY" -u "$I/latency.py" lm --model "$GGUF/$S135.gguf" --gate-wait "${GATE_WAIT:-300}" ;;
esac; done
echo finished
