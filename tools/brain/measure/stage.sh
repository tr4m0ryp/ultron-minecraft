#!/usr/bin/env bash
# Runs a fake-world relay and stages a scenario into it, then prints the report.
#   tools/brain/measure/stage.sh <scenario> <minutes> <sessions> [label]
# Uses BRAIN_* env like run.sh. Spends one model session per stretch.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../../.." && pwd)"
before="$(ls -d "$REPO"/logs/brain/2*/ 2>/dev/null | tail -1)"
"$REPO/tools/brain/run.sh" --fake --minutes "$2" --sessions "$3" &
relay=$!
for _ in $(seq 120); do
  run="$(ls -d "$REPO"/logs/brain/2*/ 2>/dev/null | tail -1)"
  [[ -n "$run" && "$run" != "$before" ]] && break
  sleep 0.5
done
echo "${4:-run} $run"
python3 "$HERE/drive.py" "$run" "$1" &
driver=$!
wait "$relay"
kill "$driver" 2>/dev/null
python3 "$HERE/report.py" "$run"
