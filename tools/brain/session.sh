#!/usr/bin/env bash
# One bounded player session: a headless harness run from the repo root that sees the
# game only through the minecraft MCP tools and may write only journal/. Called by run.sh.
#   session.sh <run-dir> <n> <socket> <stretch-min> <max-turns> <previous-ending>
# Optional env: BRAIN_EFFORT (low..max), BRAIN_MODEL, BRAIN_TOOL_TIMEOUT_MS, BRAIN_KICKOFF
# (a kickoff prompt file other than brain/kickoff.txt, for staged tests).
# Writes <run-dir>/session-<n>.jsonl (stream-json) and session-<n>.err; exits with claude's
# status (124 when the wall-clock backstop cut it).
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
RUN="$1"; N="$2"; SOCK="$3"; STRETCH="$4"; TURNS="$5"; PREV="$6"
JOURNAL="$REPO/journal"

jq -n --arg cmd "$REPO/target/release/brain-link-mcp" --arg sock "$SOCK" \
  --argjson to "${BRAIN_TOOL_TIMEOUT_MS:-0}" '{mcpServers: {minecraft: ({
    type: "stdio", command: $cmd, args: [], env: {ULTRON_BRAIN_SOCK: $sock}}
    + (if $to > 0 then {timeout: $to} else {} end))}}' > "$RUN/mcp.json"
jq -n --arg guard "$HERE/hooks/guard.sh" --arg clock "$HERE/hooks/clock.sh" '{hooks: {
  PreToolUse: [{matcher: "Read|Write|Edit", hooks: [{type: "command", command: $guard}]}],
  PostToolUse: [{matcher: "mcp__minecraft__.*", hooks: [{type: "command", command: $clock}]}]}}' \
  > "$RUN/settings.json"
echo $(( $(date +%s) + STRETCH * 60 )) > "$RUN/deadline"

size=$(cat "$JOURNAL"/*.txt 2>/dev/null | wc -c)
prompt="$(sed -e "s|{N}|$N|g" -e "s|{NOW}|$(date '+%Y-%m-%d %H:%M')|g" \
  -e "s|{PREV}|$PREV|g" -e "s|{MIN}|$STRETCH|g" "${BRAIN_KICKOFF:-$REPO/brain/kickoff.txt}")"
if (( size > 6000 )); then
  prompt+=$'\n'"The journal is $size characters, over its budget: prune it during this session."
fi
system="$(cat "$REPO/brain/player.txt" "$REPO/brain/loop.txt" "$REPO/brain/journal.txt")"

args=(-p "$prompt" --append-system-prompt "$system"
  --mcp-config "$RUN/mcp.json" --strict-mcp-config --settings "$RUN/settings.json"
  --tools "Read,Write,Edit"
  --allowedTools "mcp__minecraft" "Read(./journal/**)" "Edit(./journal/**)" "Write(./journal/**)"
  --permission-mode dontAsk --max-turns "$TURNS" --no-session-persistence
  --output-format stream-json --verbose --name "ultron-brain-$N")
[[ -n "${BRAIN_EFFORT:-}" ]] && args+=(--effort "$BRAIN_EFFORT")
[[ -n "${BRAIN_MODEL:-}" ]] && args+=(--model "$BRAIN_MODEL")

# Run as if from a plain terminal: drop variables a parent harness session sets.
clean=(-u CLAUDECODE -u CLAUDE_CODE_SESSION_ID -u CLAUDE_CODE_MESSAGING_SOCKET
  -u CLAUDE_CODE_MESSAGING_TOKEN -u CLAUDE_CODE_BRIDGE_SESSION_ID -u CLAUDE_PID
  -u CLAUDE_CODE_CHILD_SESSION -u CLAUDE_CODE_SESSION_ATTENDED -u CLAUDE_EFFORT -u AI_AGENT
  -u CLAUDE_CODE_ENTRYPOINT -u CLAUDE_CODE_EXECPATH)
# Copies stream-json lines to <base>.jsonl and their arrival time (unix ms) to <base>.ts.
stamp() {
  local line
  while IFS= read -r line; do
    printf '%s\n' "$line" >> "$1.jsonl"
    date +%s%3N >> "$1.ts"
  done
}
cd "$REPO"
date +%s%3N > "$RUN/session-$N.start"
BRAIN_RUN_DIR="$RUN" BRAIN_JOURNAL="$JOURNAL" ENABLE_TOOL_SEARCH=false \
  timeout --signal=TERM --kill-after=30 $(( STRETCH * 60 + 300 )) \
  env "${clean[@]}" claude "${args[@]}" < /dev/null \
  > >(stamp "$RUN/session-$N") 2> "$RUN/session-$N.err" &
echo $! > "$RUN/session.pid"
wait $!
status=$?
date +%s%3N > "$RUN/session-$N.end"
exit $status
