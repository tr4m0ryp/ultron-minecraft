#!/usr/bin/env bash
# Stage the screenshot: Ultron on an obsidian pillar in the End, sword up, facing the dragon,
# on a throwaway HARDCORE server (port 25566, separate from the main server on 25565).
#
# Steps for the human (workstation = the machine running the server; replace <alias> with the
# ssh alias of the workstation):
#   1. workstation:   tools/stage/end_tower.sh server start
#   2. workstation:   tools/stage/end_tower.sh bot start         (Ultron joins, ~10 s)
#   3. workstation:   tools/stage/end_tower.sh setup             (bot to the pillar, kit, facing)
#   4. your computer: ssh -N -L 25566:127.0.0.1:25566 <alias>    (leave it running)
#   5. game client:   Multiplayer > Direct Connection > 127.0.0.1:25566, name tr4m0ryp
#   6. workstation:   tools/stage/end_tower.sh camera            (you land on an invisible
#                     platform beside the pillar looking at Ultron; "camera safe" adds
#                     resistance and slow falling but shows two effect icons in the HUD)
#   7. workstation:   tools/stage/end_tower.sh say               (Ultron speaks the line, once)
#   8. game client:   press F2 (screenshot, saved by the client)
# Rebuild the scene: tools/stage/end_tower.sh reset   (full re-roll of the End, new dragon:
# reset hard; it restarts the server and the bot). Stop everything: bot stop, server stop.
# Other: status (positions, health, dragon), face (re-aim the bot at the dragon).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
STAGE="${STAGE_DIR:-$HOME/minecraft/stage-hardcore}"
BOT="${STAGE_BOT:-Ultron}"
# Only the edge and void guards run: the other reflexes turn the head toward threats and
# followed players, which ruins the pose. STAGE_MOTOR_ONLY="" restores the full catalog.
[[ -n "${STAGE_MOTOR_ONLY-R5,R7}" ]] && export MOTOR_ONLY="${STAGE_MOTOR_ONLY-R5,R7}"
# The spoken line is personal, so it lives in an untracked file next to this script.
LINE_FILE="$(dirname "$0")/line.local.txt"
LINE="${STAGE_LINE:-$( [ -f "$LINE_FILE" ] && head -n 1 "$LINE_FILE" )}"
[ -n "$LINE" ] || LINE="Alone, each of us is half a player. This time we do not fall."
py() { (cd "$HERE" && python3 tower.py "$@"); }
rcon_up() { (exec 3<>/dev/tcp/127.0.0.1/25576) 2>/dev/null; }
server_pid() { pgrep -f '^[^ ]*java .*-Xmx3G -jar server.jar' | head -1; }

server_start() {
  if rcon_up; then echo "staging server already running"; return; fi
  (cd "$STAGE" && setsid nohup ./start.sh > server.out 2>&1 < /dev/null &)
  for _ in $(seq 1 90); do
    grep -q 'RCON running' "$STAGE/server.out" 2>/dev/null && rcon_up && { echo "staging server up on 127.0.0.1:25566"; return; }
    sleep 1
  done
  echo "server did not come up; see $STAGE/server.out" >&2; exit 1
}

server_stop() {
  rcon_up || { echo "staging server not running"; return; }
  py stop_server || true
  for _ in $(seq 1 90); do [[ -z "$(server_pid)" ]] && { echo "staging server stopped"; return; }; sleep 1; done
  echo "server still up after 90 s" >&2; exit 1
}

case "${1:-setup}" in
server)
  case "${2:-}" in
  start) server_start ;;
  stop) server_stop ;;
  *) echo "usage: end_tower.sh server start|stop" >&2; exit 2 ;;
  esac ;;
bot) "$HERE/bot.sh" "${2:?start|stop}" "$BOT" ;;
setup) py setup ;;
face) py face ;;
camera) py camera "${2:-}" ;;
status) py status ;;
say)
  py face
  "$HERE/bot.sh" say "$BOT" "say $LINE" ;;
reset)
  if [[ "${2:-}" == hard ]]; then
    "$HERE/bot.sh" stop "$BOT" || true
    server_stop
    rm -rf "$STAGE/world/dimensions/minecraft/the_end" "$STAGE/world/players/data"
    server_start
    "$HERE/bot.sh" start "$BOT"
    for _ in $(seq 1 30); do py status 2>/dev/null | grep -q 'bot pos \[' && break; sleep 1; done
  fi
  py reset ;;
*) sed -n '2,22p' "$0" >&2; exit 2 ;;
esac
