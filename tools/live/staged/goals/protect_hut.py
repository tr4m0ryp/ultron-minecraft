"""Protection of the player's own blocks, live: a sealed dirt hut the bot builds itself, left
by go_to and by the no-goal escape drive; a restart that must remember the hut; and a wall
the companion places across a corridor (see protect.py for the usage).

Hut: 5x5, walls 2 high (y+1, y+2) and a roof at y+3, centred on the site; 57 cells. BEFORE
runs rebuild it by RCON each trial (the baseline remembers nothing); AFTER runs let the bot
build it (place/fill goals) and rebuild by the bot only when a trial broke part of it
(HUT_REBUILD=1: before every trial, to measure the building itself).
Broken hut cells are counted over RCON.
"""
import os, subprocess
from kit import *

HX0, HX1, HZ0, HZ1 = CX - 2, CX + 2, CZ - 2, CZ + 2
ROOF = Y + 3
CELLS = [(x, y, z) for y in (Y + 1, Y + 2) for x in range(HX0, HX1 + 1) for z in range(HZ0, HZ1 + 1)
         if x in (HX0, HX1) or z in (HZ0, HZ1)] + \
        [(x, ROOF, z) for x in range(HX0, HX1 + 1) for z in range(HZ0, HZ1 + 1)]
KIT = [("hotbar.0", "iron_pickaxe", 1), ("hotbar.1", "dirt", 16)]
EXIT = (CX + 10.5, Y + 1, CZ + 0.5)
SH = str(REPO / "tools/live/bot.sh")
BOT_BUILT = False


def build_mode():
    return os.environ.get("HUT_BUILD", "bot")


def missing():
    return [(x, y, z) for x, y, z in CELLS if not is_block(x, y, z, "dirt")]


def broken():
    return len(missing())


def newly_broken(before):
    """Hut cells that stood before the trial and are gone after it."""
    return len(set(missing()) - set(before))


def reset_site():
    """Plain floor and open air around the hut footprint (tunnels and pillars gone)."""
    rc(f"fill {X0} {Y - 3} {Z0} {X1} {Y} {Z1} stone replace air",
       f"fill {X0} {Y + 1} {Z0} {HX0 - 1} {Y + 12} {Z1} air",
       f"fill {HX1 + 1} {Y + 1} {Z0} {X1} {Y + 12} {Z1} air",
       f"fill {HX0} {Y + 1} {Z0} {HX1} {Y + 12} {HZ0 - 1} air",
       f"fill {HX0} {Y + 1} {HZ1 + 1} {HX1} {Y + 12} {Z1} air",
       f"fill {HX0} {Y + 4} {HZ0} {HX1} {Y + 12} {HZ1} air",
       f"kill @e[type=item,x={CX},y={Y},z={CZ},distance=..60]")


def hut_by_rcon():
    rc(f"fill {HX0} {Y + 1} {HZ0} {HX1} {ROOF} {HZ1} dirt",
       f"fill {HX0 + 1} {Y + 1} {HZ0 + 1} {HX1 - 1} {Y + 2} {HZ1 - 1} air")


def hut_by_bot():
    """The bot builds the hut from outside with fill goals, then is moved inside by the
    server. Returns the failure texts (empty when every goal succeeded)."""
    reset_site()
    rc(f"fill {HX0} {Y + 1} {HZ0} {HX1} {ROOF} {HZ1} air")
    prep(items=[("hotbar.1", "dirt", 64), ("hotbar.2", "dirt", 64), ("hotbar.0", "iron_pickaxe", 1)],
         at=(CX + 6.5, Y + 1, CZ - 6.5))
    specs = [f"place dirt {HX0} {Y + 1} {HZ0} {HX1} {Y + 2} {HZ0}",
             f"place dirt {HX0} {Y + 1} {HZ1} {HX1} {Y + 2} {HZ1}",
             f"place dirt {HX0} {Y + 1} {HZ0 + 1} {HX0} {Y + 2} {HZ1 - 1}",
             f"place dirt {HX1} {Y + 1} {HZ0 + 1} {HX1} {Y + 2} {HZ1 - 1}",
             f"place dirt {HX0} {ROOF} {HZ0} {HX1} {ROOF} {HZ1}"]
    fails = []
    for attempt in range(3):
        for spec in specs:
            n = task(spec)
            kind, text, _ = outcome(n, 240, "place")
            if kind != "goal_done":
                fails.append(f"{spec}: {kind} {text}")
        time.sleep(3.0)
        if not missing():
            break
        fails.append(f"attempt {attempt + 1}: missing {missing()}")
    return fails


def ensure_hut():
    """The hut as it should stand before a trial; returns notes on how it got there."""
    global BOT_BUILT
    if build_mode() == "rcon":
        reset_site()
        hut_by_rcon()
        return []
    if BOT_BUILT and broken() == 0 and os.environ.get("HUT_REBUILD") != "1":
        reset_site_outside()
        return []
    notes = hut_by_bot()
    BOT_BUILT = broken() == 0
    return [f"hut built by bot, {broken()} of {len(CELLS)} cells missing"] + notes


def reset_site_outside():
    rc(f"fill {X0} {Y - 3} {Z0} {X1} {Y} {Z1} stone replace air",
       f"kill @e[type=item,x={CX},y={Y},z={CZ},distance=..60]")


def inside():
    rc(f"tp {NAME} {CX + 0.5} {Y + 1} {CZ + 0.5}")


def in_hut(p):
    return p and HX0 <= p[0] < HX1 + 1 and HZ0 <= p[2] < HZ1 + 1 and p[1] <= Y + 3


def prep_inside():
    prep(items=KIT, at=(CX + 0.5, Y + 1, CZ + 0.5), comp_at=(CX - 12.5, Y + 1, CZ - 12.5))


def hut_go_to():
    notes = ensure_hut()
    before = missing()
    prep_inside()
    t0 = time.time()
    n = task("go_to %s %s %s" % EXIT)
    kind, text, _ = outcome(n, 150, "go_to")
    p = pos()
    b = newly_broken(before)
    return dict(ok=b == 0 and not before, hut_broken=b, sealed=not before, outcome=kind, text=text,
                secs=round(time.time() - t0, 1), reached=bool(p and dist(p, EXIT) <= 2), pos=p, notes=notes)


def hut_escape(full=False):
    """The trial ends when the server shows the body outside the hut footprint, which is
    about half a second before the escape itself is done: the next trial then teleports a
    body that is still walking out (live, that is what made 1 escape in 5 give up). With
    `full` it also waits for the body's own "out under open sky" and for the server to show
    it standing on the floor outside, so every trial starts from a finished escape."""
    notes = ensure_hut()
    before = missing()
    prep_inside()
    n = len(log_lines())
    t0, result, p = time.time(), "timeout", None
    while time.time() - t0 < 170:
        time.sleep(2)
        p = pos()
        lines = " ".join(l for l in log_lines()[n:] if " escape " in l)
        done = not full or ("out under open sky" in lines and p and p[1] >= Y + 1)
        if p and not in_hut(p) and done:
            result = "left"
            break
        if "could not dig out" in lines:
            result = "gave_up"
            break
    cells = sorted(set(missing()) - set(before))
    return dict(ok=not cells and not before, hut_broken=len(cells), sealed=not before, result=result,
                secs=round(time.time() - t0, 1), pos=p, notes=notes, broken_cells=cells)


def restart_bot(binary):
    env = dict(os.environ)
    if binary:
        env["MOTOR_BIN"] = str(binary)
    env.pop("MOTOR_ONLY", None)
    subprocess.run(["bash", SH, "stop", "bot"], check=False)
    time.sleep(2.5)
    subprocess.run(["bash", SH, "start", "bot", NAME, COMP], env=env, check=True)
    wait_joined(NAME)


def hut_restart():
    notes = ensure_hut()
    before = missing()
    time.sleep(3.0)
    binary = os.environ.get("MOTOR_BIN")
    restart_bot(binary)
    loaded = next((l for l in log_lines() if l.startswith("LEDGER ")), "no LEDGER line")
    prep_inside()
    n = task("go_to %s %s %s" % EXIT)
    kind, text, _ = outcome(n, 150, "go_to")
    b = newly_broken(before)
    return dict(ok=b == 0 and not before, hut_broken=b, sealed=not before, outcome=kind, text=text, ledger_line=loaded, notes=notes)


def dug_cells(since):
    """Cells the hand dug since log line `since` (the trace's `hand=dig:x,y,z`)."""
    out = set()
    for l in log_lines()[since:]:
        if l.startswith("T ") and "hand=dig:" in l:
            out.add(tuple(int(v) for v in l.rsplit("hand=dig:", 1)[1].split()[0].split(",")))
    return out


def hut_build():
    """The building itself: walls and roof by five fill goals, one pass, then 15 s with no
    goal. Counted on the server: hut cells standing, and floor blocks gone (the repair fill
    says how many it put back). From the trace: hut cells and floor cells the hand dug."""
    reset_site()
    rc(f"fill {HX0} {Y + 1} {HZ0} {HX1} {ROOF} {HZ1} air")
    prep(items=[("hotbar.1", "dirt", 64), ("hotbar.2", "dirt", 64), ("hotbar.0", "iron_pickaxe", 1)],
         at=(CX + 6.5, Y + 1, CZ - 6.5), comp_at=(CX - 12.5, Y + 1, CZ - 12.5))
    n0, t0, texts = len(log_lines()), time.time(), []
    for spec in (f"place dirt {HX0} {Y + 1} {HZ0} {HX1} {Y + 2} {HZ0}",
                 f"place dirt {HX0} {Y + 1} {HZ1} {HX1} {Y + 2} {HZ1}",
                 f"place dirt {HX0} {Y + 1} {HZ0 + 1} {HX0} {Y + 2} {HZ1 - 1}",
                 f"place dirt {HX1} {Y + 1} {HZ0 + 1} {HX1} {Y + 2} {HZ1 - 1}",
                 f"place dirt {HX0} {ROOF} {HZ0} {HX1} {ROOF} {HZ1}"):
        n = task(spec)
        kind, text, _ = outcome(n, 300, "place")
        texts.append(f"{kind}: {text}")
    secs = round(time.time() - t0, 1)
    p = pos()
    time.sleep(15)
    gone = missing()
    out = rc(f"fill {X0} {Y - 3} {Z0} {X1} {Y} {Z1} stone replace air")
    m = re.search(r"filled (\d+) block", out)
    floor = int(m.group(1)) if m else 0
    dug = dug_cells(n0)
    hut_dug = sorted(c for c in dug if c in CELLS)
    floor_dug = sorted(c for c in dug if c[1] <= Y)
    return dict(ok=not gone and floor == 0 and not hut_dug and not floor_dug, placed=len(CELLS) - len(gone),
                of=len(CELLS), floor_blocks_gone=floor, hut_cells_dug=len(hut_dug), floor_cells_dug=len(floor_dug),
                inside_at_end=bool(in_hut(p)), secs=secs, roof=texts[-1], pos=p)


def cleanup():
    rc(f"fill {X0 - 2} {Y + 1} {Z0 - 2} {X1 + 2} {Y + 14} {Z1 + 2} air",
       f"fill {X0} {Y - 3} {Z0} {X1} {Y} {Z1} stone",
       f"kill @e[type=item,x={CX},y={Y},z={CZ},distance=..60]")


# --- the companion's wall -------------------------------------------------------------

WZ = CZ + 12
WALL = [(CX, y, z) for y in (Y + 1, Y + 2) for z in range(WZ - 1, WZ + 2)]


def corridor():
    """Open-topped obsidian trough, 3 wide: x CX-9..CX+9. Nothing in it is diggable but
    whatever the companion puts across it."""
    rc(f"fill {CX - 9} {Y} {WZ - 2} {CX + 9} {Y + 6} {WZ + 2} obsidian",
       f"fill {CX - 8} {Y + 1} {WZ - 1} {CX + 8} {Y + 6} {WZ + 1} air")


def start_companion(binary):
    env = dict(os.environ, ULTRON_BRIDGE="0")
    if binary:
        env["MOTOR_BIN"] = str(binary)
    env.pop("MOTOR_ONLY", None)
    subprocess.run(["bash", SH, "stop", "comp"], check=False)
    time.sleep(2.5)
    subprocess.run(["bash", SH, "start", "comp", COMP], env=env, check=True)
    wait_joined(COMP)


def companion_wall():
    binary = os.environ.get("MOTOR_BIN")
    cleanup()
    corridor()
    start_companion(binary)
    prep(items=[("hotbar.0", "iron_pickaxe", 1)], at=(CX - 6.5, Y + 1, WZ + 0.5))
    rc(f"gamemode creative {COMP}", f"clear {COMP}", f"item replace entity {COMP} hotbar.1 with dirt 16",
       f"tp {COMP} {CX - 2.5} {Y + 1} {WZ + 0.5}")
    time.sleep(2.0)
    send(f"task place dirt {WALL[0][0]} {WALL[0][1]} {WALL[0][2]} {WALL[-1][0]} {WALL[-1][1]} {WALL[-1][2]}", "comp")
    t0 = time.time()
    while time.time() - t0 < 90 and any(not is_block(x, y, z, "dirt") for x, y, z in WALL):
        time.sleep(2)
    built = sum(1 for x, y, z in WALL if is_block(x, y, z, "dirt"))
    rc(f"tp {COMP} {CX + 3.5} {Y + 1} {WZ + 0.5}")
    time.sleep(4.0)
    try:
        rows = [l.split()[1:4] for l in open(f"{S}/bot.placed") if l.startswith("companion ")]
        seen = sum(1 for r in rows if tuple(int(v) for v in r) in WALL)
        others = len(rows) - seen
    except FileNotFoundError:
        seen = others = 0
    goal = (CX + 5.5, Y + 1, WZ + 0.5)
    n = task("go_to %s %s %s" % goal)
    kind, text, _ = outcome(n, 120, "go_to")
    left = sum(1 for x, y, z in WALL if is_block(x, y, z, "dirt"))
    p = pos()
    return dict(ok=left == built and built > 0, wall_built=built, wall_left=left, wall_cells_seen=seen, other_cells_seen=others,
                outcome=kind, text=text, pos=p, crossed=bool(p and p[0] > CX + 0.5))


CASES = dict(hut_build=hut_build, hut_go_to=hut_go_to, hut_escape=hut_escape, hut_restart=hut_restart,
             hut_escape_full=lambda: hut_escape(full=True), companion_wall=companion_wall)
