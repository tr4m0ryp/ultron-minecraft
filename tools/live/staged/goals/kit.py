"""Goal-layer staging kit: the goal site, the companion bot, goal commands and outcomes.

Site: a 41x41 stone floor at y=100, x 1400..1440, z 1000..1040 (force-loaded), air above.
The bot `Ultron` follows the companion `Buddy`, a second motor-azalea client with every
behaviour off (MOTOR_ONLY=none) and no brain socket, moved by RCON teleports and stdin chat.
Outcomes are read from the bot log (WAKE goal_* lines) and verified over RCON.
"""
import os, re, subprocess, sys, time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from arena import NAME, REPO, rc, send, rows, events, first, now_ms, S  # noqa: F401

# SITE_DX / SITE_DZ shift the site and BOT_NAME / BOT_COMP rename the bots, so several
# agents can stage at once on one server without touching each other.
DX, DZ = int(os.environ.get("SITE_DX", "0")), int(os.environ.get("SITE_DZ", "0"))
X0, X1, Z0, Z1, Y = 1400 + DX, 1440 + DX, 1000 + DZ, 1040 + DZ, 100
CX, CZ = 1420 + DX, 1020 + DZ
COMP = os.environ.get("BOT_COMP", "Buddy")
# STAGE_TIME=0 leaves the clock alone (the time is server-wide; another site may need night).
SET_TIME = os.environ.get("STAGE_TIME", "1") != "0"
WAKE_RE = re.compile(r"^WAKE t=(\d+) (goal_\w+) (\S+) #(\d+) \S+: (.*)$")


def build_site():
    rc("gamerule spawn_mobs false", f"forceload add {X0 - 16} {Z0 - 16} {X1 + 16} {Z1 + 16}",
       f"fill {X0} {Y - 3} {Z0} {X1} {Y} {Z1} stone",
       f"fill {X0} {Y + 1} {Z0} {X1} {Y + 12} {Z0 + 20} air",
       f"fill {X0} {Y + 1} {Z0 + 21} {X1} {Y + 12} {Z1} air",
       f"kill @e[type=!player,x={CX},y={Y},z={CZ},distance=..60]")


def start(binary=None, companion=True):
    env = dict(os.environ)
    if binary:
        env["MOTOR_BIN"] = str(binary)
    env.pop("MOTOR_ONLY", None)
    sh = str(REPO / "tools/live/bot.sh")
    if companion:
        cenv = dict(env, MOTOR_ONLY="none", ULTRON_BRIDGE="0")
        subprocess.run(["bash", sh, "start", "comp", COMP], env=cenv, check=True)
        wait_joined(COMP)
    subprocess.run(["bash", sh, "start", "bot", NAME, COMP], env=env, check=True)
    wait_joined(f"{NAME}")


def wait_joined(name):
    for _ in range(60):
        time.sleep(0.5)
        if name in rc("list"):
            time.sleep(2.0)
            return
    raise SystemExit(f"{name} did not join")


def stop():
    for who in ("bot", "comp"):
        subprocess.run(["bash", str(REPO / "tools/live/bot.sh"), "stop", who], check=False)
    time.sleep(2.0)


def prep(items=(), night=False, at=(CX + 0.5, Y + 1, CZ + 0.5), comp_at=None):
    """Clean survival bot with `items` [(slot, item, n)], placed at `at`."""
    rc(f"effect clear {NAME}", f"gamemode survival {NAME}", f"clear {NAME}",
       f"effect give {NAME} instant_health 1 5", f"effect give {NAME} saturation 1 20",
       *([f"time set {'midnight' if night else 'noon'}"] if night or SET_TIME else []),
       f"kill @e[type=!player,x={CX},y={Y},z={CZ},distance=..60]",
       *[f"item replace entity {NAME} {s} with {i} {n}" for s, i, n in items],
       f"tp {NAME} {at[0]} {at[1]} {at[2]}")
    if comp_at:
        rc(f"gamemode creative {COMP}", f"tp {COMP} {comp_at[0]} {comp_at[1]} {comp_at[2]}")
    send("goal-end")
    time.sleep(1.5)


def log_lines():
    with open(f"{S}/bot.log", errors="replace") as f:
        return f.read().splitlines()


def task(spec):
    """Sets a goal by goal-layer spec over stdin; returns the log line count before it."""
    n = len(log_lines())
    send(f"task {spec}")
    return n


def outcome(since, timeout, label=None):
    """Waits for goal_done/goal_failed after log line `since`; returns (kind, text, wakes)."""
    end = time.time() + timeout
    while time.time() < end:
        wakes = [m.groups() for l in log_lines()[since:] if (m := WAKE_RE.match(l))]
        for t, kind, subj, gid, text in wakes:
            if kind in ("goal_done", "goal_failed") and (label is None or subj == label):
                return kind, text, wakes
        time.sleep(0.5)
    return "timeout", None, [m.groups() for l in log_lines()[since:] if (m := WAKE_RE.match(l))]


def pos(name=NAME):
    out = rc(f"data get entity {name} Pos")
    v = re.findall(r"(-?[\d.]+)d", out)
    return tuple(round(float(a), 2) for a in v[:3]) if len(v) >= 3 else None


def block(x, y, z):
    """Block id at x y z, read via `execute if block` over a short candidate list."""
    out = rc(f"execute if block {x} {y} {z} minecraft:air")
    return "air" if "passed" in out.lower() else "not_air"


def is_block(x, y, z, kind):
    return "passed" in rc(f"execute if block {x} {y} {z} minecraft:{kind}").lower()


def count_item(name, item):
    """Count of `item` in a player's inventory, via `clear <name> <item> 0`."""
    out = rc(f"clear {name} minecraft:{item} 0")
    m = re.search(r"(\d+) (?:item|matching)", out)
    return int(m.group(1)) if m else (0 if "No items" in out or "not found" in out.lower() else out)


def dist(a, b):
    return round(sum((p - q) ** 2 for p, q in zip(a, b)) ** 0.5, 2) if a and b else None
