"""Stage the End tower scene: python3 tower.py setup|face|camera|status|reset [safe]

Driven by end_tower.sh. Everything runs over RCON against the staging server.
"""

import math
import os
import sys
import time

from stage_rcon import (END, block_is, connect, dimension_of, entity_is, floats, get_pos,
                        get_rot, gaze_error, in_end, look_angles, pillar_tops)

BOT = os.environ.get("STAGE_BOT", "Ultron")
HUMAN = os.environ.get("STAGE_HUMAN", "tr4m0ryp")
DRAGON = "@e[type=minecraft:ender_dragon,limit=1]"
WALL = 3  # barrier wall height around a standing area, blocks
CAM_DIST = float(os.environ.get("STAGE_CAM_DIST", "9"))
CAM_ANGLE = float(os.environ.get("STAGE_CAM_ANGLE", "70"))  # degrees off the "toward the island" axis


def ensure_end(r) -> None:
    """Put the bot in the End (this initialises the fight) and wait for the dragon."""
    if dimension_of(r, BOT) != END:
        in_end(r, f"tp {BOT} 100 49 0")
    for _ in range(60):
        if entity_is(r, DRAGON):
            return
        time.sleep(1)
    sys.exit("no ender dragon in the End after 60 s: killed? run: end_tower.sh reset hard")


def tallest(r):
    tops = pillar_tops(r)
    if not tops:
        sys.exit("no pillar found; is the End loaded?")
    return max(tops, key=lambda t: t[2])


def inward(cx: int, cz: int) -> tuple[float, float]:
    n = math.hypot(cx, cz) or 1.0
    return -cx / n, -cz / n


def wall_ring(r, cells: set, y: int) -> None:
    """Barrier wall (invisible in survival) on every cell whose neighbour is not in `cells`."""
    for (x, z) in cells:
        if any((x + dx, z + dz) not in cells for dx, dz in ((1, 0), (-1, 0), (0, 1), (0, -1))):
            in_end(r, f"fill {x} {y} {z} {x} {y + WALL - 1} {z} minecraft:barrier")


def pillar_cells(r, cx: int, cz: int, top: int) -> set:
    return {(x, z) for x in range(cx - 7, cx + 8) for z in range(cz - 7, cz + 8)
            if block_is(r, x, top, z, "obsidian")}


def setup(r) -> None:
    ensure_end(r)
    cx, cz, top = tallest(r)
    in_end(r, f"kill @e[type=minecraft:end_crystal,x={cx + .5},y={top + 1},z={cz + .5},distance=..8]")
    in_end(r, f"fill {cx - 5} {top + 1} {cz - 5} {cx + 5} {top + 8} {cz + 5} minecraft:air replace minecraft:iron_bars")
    in_end(r, f"fill {cx - 5} {top + 1} {cz - 5} {cx + 5} {top + 8} {cz + 5} minecraft:air replace minecraft:bedrock")
    wall_ring(r, pillar_cells(r, cx, cz, top), top + 1)
    r.cmd(f"gamemode survival {BOT}")
    for c in (f"clear {BOT}",
              f"effect clear {BOT}",
              f"effect give {BOT} minecraft:instant_health 1 10 true",
              f"effect give {BOT} minecraft:saturation infinite 0 true",
              f"effect give {BOT} minecraft:resistance infinite 4 true",
              f"effect give {BOT} minecraft:slow_falling infinite 0 true",
              f"attribute {BOT} minecraft:knockback_resistance base set 1",
              f"item replace entity {BOT} weapon.mainhand with minecraft:diamond_sword",
              f"item replace entity {BOT} weapon.offhand with minecraft:shield"):
        r.cmd(c)
    in_end(r, f"tp {BOT} {cx + .5} {top + 1} {cz + .5}")
    face(r)
    print(f"pillar ({cx},{cz}) top y={top}; {BOT} standing at {cx + .5} {top + 1} {cz + .5}")


def face(r) -> None:
    """Turn the bot toward the dragon (or the island centre if it cannot be found)."""
    me = get_pos(r, BOT)
    if me is None:
        sys.exit(f"{BOT} is not online")
    tgt = get_pos(r, DRAGON.replace("@e[", "@e[")) if entity_is(r, DRAGON) else None
    tgt = floats(in_end(r, f"data get entity {DRAGON} Pos"), 3) or [0.0, 70.0, 0.0]
    yaw, pitch = look_angles((me[0], me[1] + 1.62, me[2]), tuple(tgt))
    in_end(r, f"tp {BOT} {me[0]} {me[1]} {me[2]} {yaw} {pitch}")


def camera(r, safe: bool) -> None:
    cx, cz, top = tallest(r)
    ux, uz = inward(cx, cz)
    a = math.radians(CAM_ANGLE)
    # rotate the inward axis by CAM_ANGLE to stand beside, and a little toward, the island
    dx = ux * math.cos(a) - uz * math.sin(a)
    dz = ux * math.sin(a) + uz * math.cos(a)
    px, pz = math.floor(cx + .5 + dx * CAM_DIST), math.floor(cz + .5 + dz * CAM_DIST)
    py = top - 1  # camera feet one block below the bot's: a slightly low, heroic angle
    x0, x1, z0, z1 = px - 2, px + 2, pz - 2, pz + 2
    in_end(r, f"fill {x0} {py - 1} {z0} {x1} {py - 1} {z1} minecraft:barrier")
    in_end(r, f"fill {x0} {py} {z0} {x1} {py + WALL} {z1} minecraft:air replace minecraft:barrier")
    wall_ring(r, {(x, z) for x in range(x0, x1 + 1) for z in range(z0, z1 + 1)}, py)
    r.cmd(f"gamemode survival {HUMAN}")
    r.cmd(f"effect clear {HUMAN}")
    r.cmd(f"effect give {HUMAN} minecraft:instant_health 1 10 true")
    if safe:
        for c in ("resistance infinite 4 true", "slow_falling infinite 0 true"):
            r.cmd(f"effect give {HUMAN} minecraft:{c}")
    yaw, pitch = look_angles((px + .5, py + 1.62, pz + .5), (cx + .5, top + 2.1, cz + .5))
    in_end(r, f"tp {HUMAN} {px + .5} {py} {pz + .5} {yaw} {pitch}")
    print(f"{HUMAN} on platform ({px + .5},{py},{pz + .5}) looking at {BOT}")


def status(r) -> None:
    bot = get_pos(r, BOT)
    print("bot pos", bot, "rot", get_rot(r, BOT), "dim", dimension_of(r, BOT))
    for key in ("Health", "foodLevel", "playerGameType", "SelectedItem", "equipment"):
        print(key, r.cmd(f"data get entity {BOT} {key}").split(": ", 1)[-1][:160])
    print("dragon", in_end(r, f"data get entity {DRAGON} Pos").split(": ", 1)[-1],
          in_end(r, f"data get entity {DRAGON} DragonPhase").split(": ", 1)[-1],
          in_end(r, f"data get entity {DRAGON} Health").split(": ", 1)[-1])
    if bot and get_pos(r, HUMAN):
        eye = get_pos(r, HUMAN)
        print(f"{HUMAN} gaze error vs {BOT}: "
              f"{gaze_error((eye[0], eye[1] + 1.62, eye[2]), get_rot(r, HUMAN), (bot[0], bot[1] + 1.0, bot[2])):.1f} deg")


def reset(r) -> None:
    """Soft reset: undo a hardcore spectator state, heal, rebuild the scene."""
    for p in (BOT, HUMAN):
        r.cmd(f"gamemode survival {p}")
    setup(r)


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else "setup"
    conn = connect()
    {"setup": lambda: setup(conn), "face": lambda: face(conn), "status": lambda: status(conn),
     "reset": lambda: reset(conn), "camera": lambda: camera(conn, "safe" in sys.argv[2:])}[cmd]()
