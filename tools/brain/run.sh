#!/usr/bin/env bash
# The brain supervisor: plays as a relay of bounded headless harness sessions. Each
# session plays one stretch, keeps journal/ current and hands over; the next one reads the
# journal and carries on. Crashed or empty sessions restart with exponential backoff.
#   tools/brain/run.sh [--fake] [--minutes M] [--turns T] [--sessions S]
#     --fake       play in the staged fake world instead of the bot (tools/play.sh)
#     --minutes M  length of one stretch (default 15)
#     --turns T    hard cap on model turns per session (default 400)
#     --sessions S stop after S sessions (default: run until stopped)
#   tools/brain/run.sh stop     ask the running relay to hand over and stop
# Env: BRAIN_EFFORT, BRAIN_MODEL, BRAIN_TOOL_TIMEOUT_MS (see session.sh).
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
LOGS="$REPO/logs/brain"
mkdir -p "$LOGS"

if [[ "${1:-}" == stop ]]; then
  touch "$LOGS/STOP"
  pid="$(cat "$LOGS/run.pid" 2>/dev/null || true)"
  [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null && echo "stop requested (relay $pid hands over)" \
    || echo "no relay running; stop file left for the next start to clear"
  exit 0
fi

FAKE=0; MIN=15; TURNS=400; MAXS=0
while (( $# )); do
  case "$1" in
    --fake) FAKE=1 ;;
    --minutes) MIN="$2"; shift ;;
    --turns) TURNS="$2"; shift ;;
    --sessions) MAXS="$2"; shift ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

source "$HERE/lib/relay.sh"
source "$HERE/lib/home.sh"
check_home "$PLAY_HOME" "$REPO" && adopt_journal "$REPO/journal" "$PLAY_HOME" || exit 1
RUN="$LOGS/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$RUN"
rm -f "$LOGS/STOP"
echo $$ > "$LOGS/run.pid"
seed_journal "$PLAY_HOME/journal"
(cd "$REPO" && nice -n 19 cargo build --release -q -p brain-link -j 2) || exit 1

if (( FAKE )); then
  SOCK="$("$HERE/fake.sh" "$RUN")" || exit 1
else
  SOCK="${ULTRON_BRAIN_SOCK:-$(grep -m1 -o 'BRIDGE socket=.*' "$REPO/logs/play/bot.log" 2>/dev/null | cut -d= -f2-)}"
  [[ -S "$SOCK" ]] || { echo "no bot socket; start the bot with tools/play.sh first" >&2; exit 1; }
fi
trap 'touch "$LOGS/STOP"' INT TERM
trap 'cleanup' EXIT
if (( FAKE )); then
  export BRAIN_CLOCK=0
else
  start_clock "$RUN"
fi
echo "relay $RUN (socket $SOCK); stop with: tools/brain/run.sh stop"
relay "$RUN" "$SOCK"
