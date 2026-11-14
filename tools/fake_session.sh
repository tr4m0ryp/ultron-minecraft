#!/usr/bin/env bash
# Demo of the brain loop without the game: starts the fake executor (a scripted toy world
# behind the real bridge) and drives brain-link-mcp over stdio the way the harness would.
#   tools/fake_session.sh        BIN selects target/<BIN> (default release)
set -euo pipefail
REPO="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO"
BIN="target/${BIN:-release}"
[ "$BIN" = target/release ] && flag=--release || flag=
nice -n 19 cargo build $flag -q -p brain-link -j 2
work="$(mktemp -d)"
export ULTRON_BRAIN_SOCK="$work/brain.sock"
"$BIN/brain-link-fake" "$ULTRON_BRAIN_SOCK" --ticks 2400 2> "$work/fake.log" &
fake=$!
trap 'kill $fake 2>/dev/null; rm -rf "$work"' EXIT
for _ in $(seq 50); do [ -S "$ULTRON_BRAIN_SOCK" ] && break; sleep 0.1; done

mkfifo "$work/in" "$work/out"
"$BIN/brain-link-mcp" < "$work/in" > "$work/out" &
exec 3> "$work/in" 4< "$work/out"
id=0
rpc() { # method params-json -> prints the reply line
  id=$((id + 1))
  printf '{"jsonrpc":"2.0","id":%d,"method":"%s","params":%s}\n' "$id" "$1" "$2" >&3
  IFS= read -r reply <&4
  printf '%s\n' "$reply"
}
tool() { # name args-json -> prints the call and its text result
  printf '\n> %s %s\n' "$1" "$2"
  rpc tools/call "{\"name\":\"$1\",\"arguments\":$2}" | jq -r '.result.content[0].text'
}

rpc initialize '{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"demo","version":"0"}}' \
  | jq -c '{server: .result.serverInfo, protocol: .result.protocolVersion}'
printf '{"jsonrpc":"2.0","method":"notifications/initialized"}\n' >&3
rpc tools/list '{}' | jq -c '[.result.tools[].name]'
sleep 1.2
tool observe '{}'
tool wait_event '{"timeout_s":10}'
tool say '{"text":"on my way"}'
tool set_goal '{"goal":"come_here","priority":0.8}'
tool wait_event '{"timeout_s":10}'
tool observe '{}'
tool recent_chat '{"limit":5}'
tool set_goal '{"goal":"mine","params":{"kind":"iron_ore","count":3}}'
printf '\nfake executor stderr:\n'
cat "$work/fake.log"
