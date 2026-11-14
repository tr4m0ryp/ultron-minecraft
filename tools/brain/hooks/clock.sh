#!/usr/bin/env bash
# PostToolUse hook on the minecraft tools: once the stretch deadline has passed or a stop
# was requested, tell the player to hand over (write the journal and end the turn).
set -uo pipefail
cat > /dev/null
run="${BRAIN_RUN_DIR:?}"
deadline="$(cat "$run/deadline" 2>/dev/null || echo 0)"
if [[ -e "$run/../STOP" || -e "$run/STOP" ]]; then
  why="the human is stopping the game"
elif (( $(date +%s) >= deadline && deadline > 0 )); then
  why="this stretch is over"
else
  exit 0
fi
msg="HANDOVER: $why. Update journal/projects.txt and any other journal file that changed, add one dated line to journal/log.txt, then end your turn with a one line summary. Do not call wait_event again."
jq -cn --arg m "$msg" '{hookSpecificOutput: {hookEventName: "PostToolUse", additionalContext: $m}}'
