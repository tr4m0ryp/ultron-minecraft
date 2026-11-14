#!/usr/bin/env bash
# Start Ultron for a play session: build release if needed, connect the bot to the server
# with the brain socket mounted, and print how to start thought (the harness here).
#   tools/play.sh [companion] [host:port] [username]   start (defaults: $ULTRON_COMPANION,
#                                                      127.0.0.1:25565, Ultron)
#   tools/play.sh stop                                 stop the bot
#   tools/play.sh status                               is it running, which socket
# Stdin commands for the bot (task <spec>, goal-end, say <text>) go to logs/play/bot.in.
set -euo pipefail
REPO="$(cd "$(dirname "$0")/.." && pwd)"
RUN="$REPO/logs/play"
PID="$RUN/bot.pid"
mkdir -p "$RUN"

running() { [[ -f "$PID" ]] && kill -0 -- "-$(cat "$PID")" 2>/dev/null; }
socket_of() { grep -m1 -o 'BRIDGE socket=.*' "$RUN/bot.log" 2>/dev/null | cut -d= -f2- || true; }

case "${1:-}" in
stop)
  if running; then kill -- "-$(cat "$PID")"; echo "stopped"; else echo "not running"; fi
  rm -f "$PID"; exit 0 ;;
status)
  if running; then echo "running, socket $(socket_of)"; else echo "not running"; fi
  exit 0 ;;
esac

COMPANION="${1:-${ULTRON_COMPANION:-}}"
ADDR="${2:-127.0.0.1:25565}"
NAME="${3:-Ultron}"
if [[ -z "$COMPANION" ]]; then
  echo "usage: tools/play.sh <companion-player-name> [host:port] [username]" >&2
  echo "       (or set ULTRON_COMPANION)" >&2
  exit 2
fi
if running; then
  echo "already running (pid group $(cat "$PID")), socket $(socket_of)"; exit 0
fi

echo "building release (no-op when up to date)..."
(cd "$REPO" && cargo build --release -q -p motor-azalea -p brain-link)

: > "$RUN/bot.in"
cd "$REPO"
setsid bash -c "tail -f '$RUN/bot.in' | RUST_LOG=\${RUST_LOG:-warn} \
  target/release/motor-azalea '$ADDR' '$NAME' '$COMPANION' > '$RUN/bot.log' 2>&1" \
  < /dev/null > /dev/null 2>&1 &
echo $! > "$PID"

for _ in $(seq 1 60); do
  sock="$(socket_of)"
  [[ -n "$sock" ]] && break
  if ! running; then echo "bot exited; see $RUN/bot.log" >&2; tail -5 "$RUN/bot.log" >&2; exit 1; fi
  sleep 0.5
done
if [[ -z "${sock:-}" ]]; then
  echo "bot started but no brain socket after 30 s; see $RUN/bot.log" >&2; exit 1
fi

cat <<EOF
$NAME is joining $ADDR, playing with $COMPANION.
brain socket: $sock
log:          $RUN/bot.log      stop: tools/play.sh stop

Start the play session in a second terminal, from the repo root:
  tools/brain/run.sh
It plays in relays and keeps its own journal in journal/. Stop: tools/brain/run.sh stop
EOF
