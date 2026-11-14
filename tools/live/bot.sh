#!/usr/bin/env bash
# Run a motor-azalea bot against the local test server with the per-tick trace on.
#   bot.sh start <who> <username> [companion]   stdin <- $RUN/<who>.in, stdout -> $RUN/<who>.log
#   bot.sh stop <who>
# RUN defaults to <repo>/logs/live (gitignored); BIN selects target/<BIN> (default release),
# MOTOR_BIN overrides the binary path; MOTOR_ONLY (comma-separated ids) limits the catalog.
REPO="$(cd "$(dirname "$0")/../.." && pwd)"
RUN="${LIVE_RUN_DIR:-$REPO/logs/live}"
mkdir -p "$RUN"
who=$2
case "$1" in
start)
  : > "$RUN/$who.in"
  cd "$REPO" || exit 1
  setsid bash -c "tail -f '$RUN/$who.in' | RUST_LOG=${RUST_LOG:-warn} MOTOR_TRACE=1 \
    ${MOTOR_BIN:-target/${BIN:-release}/motor-azalea} 127.0.0.1:25565 $3 $4 > '$RUN/$who.log' 2>&1" \
    < /dev/null > /dev/null 2>&1 &
  echo $! > "$RUN/$who.pid" ;;
stop) kill -- -"$(cat "$RUN/$who.pid")" ;;
*) echo "usage: bot.sh start|stop <who> [username] [companion]" >&2; exit 2 ;;
esac
