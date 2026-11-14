# Relay loop helpers for run.sh (sourced; uses REPO, HERE, LOGS, MIN, TURNS, MAXS, FAKE).

BACKOFF_START=15
BACKOFF_MAX=600
MAX_FAILS=6
MIN_HEALTHY_S=45
STOP_GRACE_S=180

seed_journal() {
  local j="$1"
  [[ -f "$j/places.txt" ]] || echo "# name | x y z | what it is" > "$j/places.txt"
  [[ -f "$j/projects.txt" ]] || echo "# status | project | next step | notes" > "$j/projects.txt"
  [[ -f "$j/human.txt" ]] || echo "# asked|likes|dislikes | ..." > "$j/human.txt"
  [[ -f "$j/log.txt" ]] || echo "# date time | what happened this session" > "$j/log.txt"
}

# Waits up to $1 seconds, returning early when a stop is requested.
nap() {
  local end=$(( $(date +%s) + $1 ))
  while (( $(date +%s) < end )); do
    [[ -e "$LOGS/STOP" ]] && return
    sleep 2
  done
}

# Classifies a finished session: prints "ok", "empty", "limit" or "crash".
verdict() {
  local run="$1" n="$2" status="$3" start end calls
  start=$(cat "$run/session-$n.start"); end=$(cat "$run/session-$n.end")
  calls=$(jq -r 'select(.type=="assistant") | .message.content[]? | select(.type=="tool_use")
    | .name' "$run/session-$n.jsonl" 2>/dev/null | grep -c '^mcp__minecraft__')
  if (( status != 0 )) && grep -qiE 'rate.?limit|usage limit|overloaded|quota' \
       "$run/session-$n.err" <(jq -r 'select(.type=="result") | .result? // empty' \
       "$run/session-$n.jsonl" 2>/dev/null); then
    echo limit
  elif (( calls == 0 )); then
    echo empty
  elif (( status != 0 && status != 124 )) && (( (end - start) / 1000 < MIN_HEALTHY_S )); then
    echo crash
  else
    echo ok
  fi
}

relay() {
  local run="$1" sock="$2" n=0 fails=0 wait=$BACKOFF_START prev="none, this is the first session"
  local status v
  while [[ ! -e "$LOGS/STOP" ]]; do
    (( MAXS > 0 && n >= MAXS )) && break
    n=$(( n + 1 ))
    echo "$(date '+%H:%M:%S') session $n starts (stretch $MIN min)"
    "$HERE/session.sh" "$run" "$n" "$sock" "$MIN" "$TURNS" "$prev" &
    local child=$!
    local stop_seen=0
    while kill -0 "$child" 2>/dev/null; do
      sleep 2
      if [[ -e "$LOGS/STOP" ]]; then
        (( stop_seen == 0 )) && stop_seen=$(date +%s)
        if (( $(date +%s) - stop_seen > STOP_GRACE_S )); then
          echo "no handover within $STOP_GRACE_S s; ending session $n"
          kill -TERM "$(cat "$run/session.pid" 2>/dev/null)" 2>/dev/null
        fi
      fi
    done
    wait "$child"; status=$?
    v="$(verdict "$run" "$n" "$status")"
    echo "$(date '+%H:%M:%S') session $n ended: exit $status, $v"
    if [[ "$v" == ok ]]; then
      fails=0; wait=$BACKOFF_START
      if (( status == 0 )); then prev="ended with a clean handover"
      else prev="was cut off (exit $status); trust projects.txt over log.txt"; fi
      continue
    fi
    fails=$(( fails + 1 ))
    prev="failed ($v); the journal may be a step behind"
    if (( fails >= MAX_FAILS )); then
      echo "giving up after $fails failed sessions in a row; see $run" >&2; return 1
    fi
    (( MAXS > 0 && n >= MAXS )) && break
    [[ "$v" == limit ]] && (( wait < 300 )) && wait=300
    echo "backing off $wait s before the next session"
    nap "$wait"
    wait=$(( wait * 2 > BACKOFF_MAX ? BACKOFF_MAX : wait * 2 ))
  done
  echo "relay stopped after $n sessions"
}

cleanup() {
  rm -f "$LOGS/run.pid"
  if [[ -n "${RUN:-}" && -f "$RUN/fake.pid" ]]; then
    kill "$(cat "$RUN/fake.pid")" 2>/dev/null
    rm -rf "$(cat "$RUN/fake.sockdir" 2>/dev/null)"
  fi
}
