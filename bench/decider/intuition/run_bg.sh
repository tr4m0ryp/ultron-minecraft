#!/usr/bin/env bash
# Runs one script of this directory in the background with 4 math threads and a log in ../logs (ignored by git).
#   run_bg.sh <log-name> <python> <script.py> [args...]
HERE="$(cd "$(dirname "$0")" && pwd)"; mkdir -p "$HERE/../logs"
name=$1; py=$2; script=$3; shift 3
OMP_NUM_THREADS=4 OPENBLAS_NUM_THREADS=4 MKL_NUM_THREADS=4 TOKENIZERS_PARALLELISM=false \
  nohup "$py" -u "$HERE/$script" "$@" > "$HERE/../logs/$name.log" 2>&1 &
echo "$name pid $!"
