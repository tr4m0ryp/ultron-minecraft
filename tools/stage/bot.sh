#!/usr/bin/env bash
# Run a bot against the staging server (127.0.0.1:25566) with the tool bridge off, so it
# cannot collide with a bot on the main server.
#   bot.sh start [username] [companion]   stdin <- logs/stage/<username>.in
#   bot.sh stop [username]
#   bot.sh say [username] <text>          send a stdin line to the bot
REPO="$(cd "$(dirname "$0")/../.." && pwd)"
RUN="$REPO/logs/stage"
mkdir -p "$RUN"
cmd="${1:-}"; shift || true
case "$cmd" in
start)
  who="${1:-Ultron}"
  : > "$RUN/$who.in"
  cd "$REPO" || exit 1
  ULTRON_BRIDGE=0 setsid bash -c "tail -f '$RUN/$who.in' | RUST_LOG=\${RUST_LOG:-warn} \
    ULTRON_BRIDGE=0 target/release/motor-azalea 127.0.0.1:25566 '$who' ${2:+'$2'} \
    > '$RUN/$who.log' 2>&1" < /dev/null > /dev/null 2>&1 &
  echo $! > "$RUN/$who.pid"
  echo "$who joining 127.0.0.1:25566 (log $RUN/$who.log)" ;;
stop)
  who="${1:-Ultron}"
  [[ -f "$RUN/$who.pid" ]] && kill -- "-$(cat "$RUN/$who.pid")" 2>/dev/null
  rm -f "$RUN/$who.pid"; echo "$who stopped" ;;
say)
  who="${1:-Ultron}"; shift
  printf '%s\n' "$*" >> "$RUN/$who.in" ;;
*) echo "usage: bot.sh start [name] [companion] | stop [name] | say [name] <line>" >&2; exit 2 ;;
esac
