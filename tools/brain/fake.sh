#!/usr/bin/env bash
# Starts the staged fake world for a brain run and prints its socket path.
#   fake.sh <run-dir>    control file <run-dir>/stage.ctl, log <run-dir>/fake.log,
#                        pid <run-dir>/fake.pid. Append lines to stage.ctl to stage events.
set -euo pipefail
REPO="$(cd "$(dirname "$0")/../.." && pwd)"
RUN="$1"
sockdir="$(mktemp -d -p "${XDG_RUNTIME_DIR:-/tmp}" ultron-fake.XXXXXX)"
: > "$RUN/stage.ctl"
"$REPO/target/release/brain-link-fake" "$sockdir/brain.sock" --stage "$RUN/stage.ctl" \
  > /dev/null 2> "$RUN/fake.log" < /dev/null &
echo $! > "$RUN/fake.pid"
echo "$sockdir" > "$RUN/fake.sockdir"
for _ in $(seq 50); do [[ -S "$sockdir/brain.sock" ]] && break; sleep 0.1; done
[[ -S "$sockdir/brain.sock" ]] || { echo "fake world did not start; see $RUN/fake.log" >&2; exit 1; }
echo "$sockdir/brain.sock"
