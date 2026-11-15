"""Live reflex harness: stages situations over RCON and reads the bot trace (MOTOR_TRACE=1).

The test arena is a stone platform at y=149 spanning x,z 960..1040 (force-loaded).
"""
import os, re, sys, time
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from rcon import connect

S = os.environ.get("LIVE_RUN_DIR", str(Path(__file__).resolve().parents[2] / "logs" / "live"))
R = connect()
T_RE = re.compile(r"^T (\d+) ms=(\d+) pos=([-\d.]+),([-\d.]+),([-\d.]+) hp=([-\d.]+) food=(\d+) sat=([-\d.]+) air=(-?\d+) fire=(\d) water=(\d) ground=(\d) slot=(\d+) held=(.+?) ents=(\d+) legs=(\S+) gaze=(\S+) main=(\S+) inv=(\S+)")

def rc(*cmds):
    """Runs RCON commands; a connection the server dropped is reopened once per command."""
    global R
    out = []
    for c in cmds:
        try:
            out.append(R.cmd(c))
        except (ConnectionError, OSError):
            R = connect()
            out.append(R.cmd(c))
    return out[-1] if out else ""

def send(line, who="bot"):
    with open(f"{S}/{who}.in", "a") as f:
        f.write(line + "\n")

def now_ms():
    return int(time.time() * 1000)

def rows(who="bot", since_ms=0):
    out = []
    with open(f"{S}/{who}.log", errors="replace") as f:
        for line in f:
            m = T_RE.match(line)
            if m and int(m.group(2)) >= since_ms:
                g = m.groups()
                out.append(dict(tick=int(g[0]), ms=int(g[1]), x=float(g[2]), y=float(g[3]), z=float(g[4]),
                                hp=float(g[5]), food=int(g[6]), sat=float(g[7]), air=int(g[8]), fire=int(g[9]),
                                water=int(g[10]), ground=int(g[11]), slot=int(g[12]), held=g[13], ents=int(g[14]),
                                legs=g[15], gaze=g[16], main=g[17], inv=g[18]))
    return out

def events(who="bot", since_tick=0):
    out = []
    with open(f"{S}/{who}.log", errors="replace") as f:
        for line in f:
            if line.startswith(("E ", "C ", "D ", "WAKE")):
                p = line.split()
                try:
                    t = int(p[1]) if not line.startswith("WAKE") else int(p[1][2:])
                except ValueError:
                    continue
                if t >= since_tick:
                    out.append(line.rstrip())
    return out

def first(rs, pred):
    return next((r for r in rs if pred(r)), None)

def reset(x=1000.5, y=150, z=1000.5, night=False):
    rc("kill @e[type=!player,x=1000,y=150,z=1000,distance=..60]",
       "fill 990 149 990 1010 149 1010 stone", "fill 990 150 990 1010 160 1010 air",
       "fill 994 141 994 998 148 998 stone",
       "effect clear Ultron", "gamemode survival Ultron",
       "effect give Ultron instant_health 1 5", "effect give Ultron saturation 1 20",
       "clear Ultron", "item replace entity Ultron hotbar.0 with diamond_sword",
       f"time set {'midnight' if night else 'noon'}", f"tp Ultron {x} {y} {z} 0 0")
    send("goal-end")
    time.sleep(1.5)

def server_deaths(since_line=0):
    lines = open(os.path.expanduser("~/minecraft/vanilla-26.3/server.out"), errors="replace").read().splitlines()
    return [l for l in lines[since_line:] if re.search(r"Ultron (was|died|drowned|burned|fell|blew|hit|tried|walked|suffocated|starved|experienced|froze|went)", l)]

def log_len():
    return len(open(os.path.expanduser("~/minecraft/vanilla-26.3/server.out"), errors="replace").read().splitlines())

def build_arena():
    """One-off: force-load and build the platform; natural spawns off so tests stay clean."""
    rc("gamerule spawn_mobs false", "forceload add 944 944 1056 1056",
       "fill 960 149 960 1040 149 1040 stone", "fill 960 150 960 1040 160 986 air",
       "fill 960 150 987 1040 160 1013 air", "fill 960 150 1014 1040 160 1040 air",
       "fill 994 141 994 998 148 998 stone")
