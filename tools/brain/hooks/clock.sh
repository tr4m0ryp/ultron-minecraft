#!/usr/bin/env bash
# PostToolUse hook on the minecraft tools. Adds to the tool result:
#   a game clock notice when day turned to dusk, night or dawn (lib/gameclock.py), since
#   the body's own day or night word can lag;
#   once the stretch deadline has passed or a stop was requested, the HANDOVER order with
#   the real wall clock time and game day for the journal stamps.
set -uo pipefail
cat > /dev/null
run="${BRAIN_RUN_DIR:?}"
lib="$(cd "$(dirname "$0")/../lib" && pwd)"
notes=()
if [[ "${BRAIN_CLOCK:-1}" == 1 ]]; then
  n="$(python3 "$lib/gameclock.py" notice "$run" 2>/dev/null)"
  [[ -n "$n" ]] && notes+=("$n")
fi
deadline="$(cat "$run/deadline" 2>/dev/null || echo 0)"
why=""
if [[ -e "$run/../STOP" || -e "$run/STOP" ]]; then
  why="the human is stopping the game"
elif (( $(date +%s) >= deadline && deadline > 0 )); then
  why="this stretch is over"
fi
if [[ -n "$why" ]]; then
  stamp="$(python3 "$lib/gameclock.py" stamp "$run" 2>/dev/null || echo "day unknown")"
  notes+=("HANDOVER: $why. It is now $(date '+%Y-%m-%d %H:%M') (wall clock), game $stamp. Update journal/projects.txt and any other journal file that changed, add one line to journal/log.txt stamped with this session's start time from the kickoff, then end your turn with a one line summary. Do not call wait_event again.")
fi
(( ${#notes[@]} )) || exit 0
msg="$(printf '%s\n' "${notes[@]}")"
jq -cn --arg m "${msg%$'\n'}" '{hookSpecificOutput: {hookEventName: "PostToolUse", additionalContext: $m}}'
