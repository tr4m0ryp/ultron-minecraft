"""Staging helpers shared by the staged scenarios: sites, bot lifecycle, server-log parsing.

Sites (all force-loaded, away from the 81x81 harness platform at x,z 960..1040):
  edge  21x21 floating stone platform at y=149, x 1100..1120, z 990..1010 (about 80 block drop)
  pit   30x30 walled survival arena at y=100, x 1200..1229, z 1100..1129
  pool  water pool with a stone shore at y=120, x 1300..1320, z 1000..1020
"""
import os, re, subprocess, sys, time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from harness import R, rc, send, rows, events, first, log_len, now_ms, S  # noqa: F401

REPO = Path(__file__).resolve().parents[3]
SERVER_OUT = os.path.expanduser("~/minecraft/vanilla-26.3/server.out")
DEATH_RE = re.compile(r"(?:\]: |System chat: )(Ultron (?:was|died|drowned|burned|fell|blew|hit|tried|walked|suffocated|starved|experienced|froze|went|withered).*)$")
NAMED_RE = re.compile(r"Named entity .*?(w\d+[a-z]\d+).*? died: (.*)$")


def bot_start(binary=None, only=None, who="bot"):
    env = dict(os.environ)
    if binary:
        env["MOTOR_BIN"] = str(binary)
    if only is not None:
        env["MOTOR_ONLY"] = only
    else:
        env.pop("MOTOR_ONLY", None)
    subprocess.run(["bash", str(REPO / "tools/live/bot.sh"), "start", who, "Ultron"], env=env, check=True)
    for _ in range(60):
        time.sleep(0.5)
        if "Ultron" in rc("list"):
            time.sleep(2.0)
            return
    raise SystemExit("bot did not join")


def bot_stop(who="bot"):
    subprocess.run(["bash", str(REPO / "tools/live/bot.sh"), "stop", who], check=False)
    time.sleep(2.0)


def server_lines(since):
    return open(SERVER_OUT, errors="replace").read().splitlines()[since:]


def deaths(since):
    return [m.group(1) for l in server_lines(since) if (m := DEATH_RE.search(l))]


def named_deaths(since):
    return [(m.group(1), m.group(2)) for l in server_lines(since) if (m := NAMED_RE.search(l))]


def prep_bot(night=True, items=(("hotbar.0", "diamond_sword", 1),), heal=True):
    cmds = ["effect clear Ultron", "gamemode survival Ultron", "clear Ultron",
            f"time set {'midnight' if night else 'noon'}"]
    if heal:
        cmds += ["effect give Ultron instant_health 1 5", "effect give Ultron saturation 1 20"]
    cmds += [f"item replace entity Ultron {slot} with {item} {n}" for slot, item, n in items]
    rc(*cmds)
    send("goal-end")


def build_edge():
    rc("forceload add 1084 974 1136 1026",
       "fill 1084 145 974 1136 155 1026 air",
       "fill 1100 149 990 1120 149 1010 stone")
